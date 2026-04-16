use crate::types::SimTransaction;
use colored::*;
use reqwest::Client;
use serde_json::{json, Value};

/// Raydium V4 AMM program ID — most DEX swaps go through here
const RAYDIUM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Pump.fun program ID
const PUMP_FUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Vote program — we filter these OUT (they're validator votes, not user txs)
const VOTE_PROGRAM: &str = "Vote111111111111111111111111111111111111111";

/// Compute Budget program — used to set priority fees
const COMPUTE_BUDGET: &str = "ComputeBudget111111111111111111111111111111";

/// Fetch the latest confirmed slot number
async fn get_latest_slot(client: &Client, rpc_url: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSlot",
        "params": [{"commitment": "finalized"}]
    });

    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    resp["result"]
        .as_u64()
        .ok_or_else(|| format!("Failed to get slot: {:?}", resp).into())
}

/// Fetch a full block by slot number
async fn get_block(
    client: &Client,
    rpc_url: &str,
    slot: u64,
) -> Result<Value, Box<dyn std::error::Error>> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBlock",
        "params": [
            slot,
            {
                "encoding": "json",
                "commitment": "finalized",
                "maxSupportedTransactionVersion": 0,
                "transactionDetails": "full",
                "rewards": false
            }
        ]
    });

    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    if resp["result"].is_null() {
        return Err(format!("Block not found for slot {}", slot).into());
    }

    Ok(resp["result"].clone())
}

/// Extract priority fee from a transaction's instructions.
/// Priority fee is set via ComputeBudget.SetComputeUnitPrice instruction.
/// The total fee minus the base fee (5000 lamports per signature) gives us
/// the priority fee portion.
fn extract_priority_fee(tx_meta: &Value, num_signatures: u64) -> u64 {
    let total_fee = tx_meta["fee"].as_u64().unwrap_or(0);
    let base_fee = 5000 * num_signatures;
    if total_fee > base_fee {
        total_fee - base_fee
    } else {
        0
    }
}

/// Check if a transaction interacts with a specific program
fn involves_program(account_keys: &[String], program_id: &str) -> bool {
    account_keys.iter().any(|k| k == program_id)
}

/// Convert a raw Solana block into a vector of SimTransactions.
/// Filters out vote transactions and optionally filters for DEX-only.
fn parse_block_transactions(
    block: &Value,
    dex_only: bool,
) -> Vec<SimTransaction> {
    let transactions = match block["transactions"].as_array() {
        Some(txs) => txs,
        None => return Vec::new(),
    };

    let block_time = block["blockTime"].as_u64().unwrap_or(0);
    let mut sim_txs = Vec::new();

    for (idx, tx_wrapper) in transactions.iter().enumerate() {
        let meta = &tx_wrapper["meta"];
        let transaction = &tx_wrapper["transaction"];

        // Skip failed transactions
        if !meta["err"].is_null() {
            continue;
        }

        // Extract account keys
        let account_keys: Vec<String> = transaction["message"]["accountKeys"]
            .as_array()
            .map(|keys| {
                keys.iter()
                    .filter_map(|k| k.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Skip vote transactions
        if involves_program(&account_keys, VOTE_PROGRAM) {
            continue;
        }

        // If DEX-only mode, skip non-DEX transactions
        if dex_only {
            let is_dex = involves_program(&account_keys, RAYDIUM_V4)
                || involves_program(&account_keys, PUMP_FUN);
            if !is_dex {
                continue;
            }
        }

        // Extract signatures
        let signatures: Vec<String> = transaction["signatures"]
            .as_array()
            .map(|sigs| {
                sigs.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let signature = signatures.first().cloned().unwrap_or_else(|| format!("tx_{}", idx));
        let num_signatures = signatures.len() as u64;

        // The first signer is the fee payer / main signer
        let signer = account_keys.first().cloned().unwrap_or_default();

        // Extract compute units consumed
        let compute_units = meta["computeUnitsConsumed"].as_u64().unwrap_or(200_000);

        // Extract priority fee
        let priority_fee = extract_priority_fee(meta, num_signatures);

        // Determine read/write accounts from the header
        let header = &transaction["message"]["header"];
        let num_required_sigs = header["numRequiredSignatures"].as_u64().unwrap_or(1) as usize;
        let num_readonly_signed = header["numReadonlySignedAccounts"].as_u64().unwrap_or(0) as usize;
        let num_readonly_unsigned = header["numReadonlyUnsignedAccounts"].as_u64().unwrap_or(0) as usize;

        // Writable accounts = all accounts except readonly ones
        // In Solana's account key layout:
        //   [0..num_required_sigs - num_readonly_signed] = writable signers
        //   [num_required_sigs - num_readonly_signed..num_required_sigs] = readonly signers
        //   [num_required_sigs..len - num_readonly_unsigned] = writable non-signers
        //   [len - num_readonly_unsigned..len] = readonly non-signers
        let writable_signer_end = num_required_sigs.saturating_sub(num_readonly_signed);
        let readonly_unsigned_start = account_keys.len().saturating_sub(num_readonly_unsigned);

        let mut write_accounts = Vec::new();
        let mut read_accounts = Vec::new();

        for (i, key) in account_keys.iter().enumerate() {
            if i < writable_signer_end {
                write_accounts.push(key.clone());
            } else if i < num_required_sigs {
                read_accounts.push(key.clone());
            } else if i < readonly_unsigned_start {
                write_accounts.push(key.clone());
            } else {
                read_accounts.push(key.clone());
            }
        }

        // Also include loaded addresses (from address lookup tables)
        if let Some(loaded) = meta["loadedAddresses"].as_object() {
            if let Some(writable) = loaded["writable"].as_array() {
                for addr in writable {
                    if let Some(s) = addr.as_str() {
                        write_accounts.push(s.to_string());
                    }
                }
            }
            if let Some(readonly) = loaded["readonly"].as_array() {
                for addr in readonly {
                    if let Some(s) = addr.as_str() {
                        read_accounts.push(s.to_string());
                    }
                }
            }
        }

        // Use block time + index as a synthetic arrival time
        // In reality, arrival time would come from the mempool/TPU ingress
        let arrival_time_us = block_time * 1_000_000 + (idx as u64 * 1_000);

        // Detect if this might be part of a Jito bundle
        // Heuristic: transactions with very high priority fees that share
        // write accounts with adjacent transactions may be bundled
        let is_bundle = false; // We'll detect this in post-processing
        
        // Create a short label from the signature
        let short_sig = if signature.len() > 8 {
            &signature[..8]
        } else {
            &signature
        };

        let is_raydium = involves_program(&account_keys, RAYDIUM_V4);
        let is_pump = involves_program(&account_keys, PUMP_FUN);
        let label = if is_raydium {
            format!("Raydium swap ({})", short_sig)
        } else if is_pump {
            format!("Pump.fun tx ({})", short_sig)
        } else {
            format!("tx ({})", short_sig)
        };

        sim_txs.push(SimTransaction {
            signature,
            signer,
            read_accounts,
            write_accounts,
            base_fee_lamports: 5_000 * num_signatures,
            priority_fee_lamports: priority_fee,
            compute_units,
            arrival_time_us,
            is_bundle,
            bundle_id: None,
            bundle_position: None,
            label: Some(label),
        });
    }

    sim_txs
}

/// Detect MEV patterns in a block:
/// - Sandwich: A-B-C where A and C share signer, B is different, all touch same accounts
/// - Coordinated buys: multiple different signers hitting same pool in sequence with high fees
/// - Repeat signer: same wallet appearing multiple times in one block
/// - Fee disparity: someone paying 10x+ the average priority fee
/// - Frontrun: high-fee tx arriving after but placed before a low-fee tx on same accounts
pub fn detect_mev_patterns(txs: &mut Vec<SimTransaction>) -> Vec<crate::types::MevEvent> {
    use crate::types::{MevEvent, MevType};
    use std::collections::{HashMap, HashSet};
    
    let mut events = Vec::new();
    let mut bundle_counter = 0u32;

    // --- Pattern 1: Sandwich detection (widened window) ---
    // Look for A...B...C where A and C share signer within 5 positions
    for i in 0..txs.len() {
        for gap in 2..6.min(txs.len() - i) {
            let c_idx = i + gap;
            if c_idx >= txs.len() { break; }
            
            if txs[i].signer == txs[c_idx].signer {
                // Check if any tx between them is a different signer touching same accounts
                let i_writes: HashSet<&str> = txs[i].write_accounts.iter().map(|s| s.as_str()).collect();
                let c_writes: HashSet<&str> = txs[c_idx].write_accounts.iter().map(|s| s.as_str()).collect();
                let ic_overlap: Vec<&&str> = i_writes.intersection(&c_writes).collect();
                
                if ic_overlap.is_empty() { continue; }
                
                for victim_idx in (i + 1)..c_idx {
                    if txs[victim_idx].signer != txs[i].signer {
                        let v_writes: HashSet<&str> = txs[victim_idx].write_accounts.iter().map(|s| s.as_str()).collect();
                        let overlap: Vec<&&str> = i_writes.intersection(&v_writes).collect();
                        if !overlap.is_empty() {
                            let bundle_id = format!("sandwich_{}", bundle_counter);
                            bundle_counter += 1;
                            txs[i].is_bundle = true;
                            txs[i].bundle_id = Some(bundle_id.clone());
                            txs[i].bundle_position = Some(0);
                            txs[c_idx].is_bundle = true;
                            txs[c_idx].bundle_id = Some(bundle_id);
                            txs[c_idx].bundle_position = Some(2);
                            
                            let extracted = txs[i].priority_fee_lamports + txs[c_idx].priority_fee_lamports;
                            events.push(MevEvent {
                                event_type: MevType::Sandwich,
                                extractor_tx: txs[i].signature.clone(),
                                victim_tx: txs[victim_idx].signature.clone(),
                                value_extracted_lamports: extracted,
                                description: format!(
                                    "Sandwich: {} wraps {} on shared accounts",
                                    &txs[i].signer[..8],
                                    &txs[victim_idx].signer[..8],
                                ),
                            });
                            break; // found a victim for this pair
                        }
                    }
                }
            }
        }
    }

    // --- Pattern 2: Repeat signer (same wallet, multiple txs in block) ---
    let mut signer_counts: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, tx) in txs.iter().enumerate() {
        signer_counts.entry(tx.signer.clone()).or_default().push(i);
    }
    let repeat_signers: Vec<(String, Vec<usize>)> = signer_counts
        .into_iter()
        .filter(|(_, indices)| indices.len() >= 2)
        .collect();
    for (signer, indices) in &repeat_signers {
        for &idx in indices {
            txs[idx].is_bundle = true;
            txs[idx].bundle_id = Some(format!("repeat_{}", &signer[..8.min(signer.len())]));
        }
        events.push(MevEvent {
            event_type: MevType::RepeatSigner,
            extractor_tx: txs[indices[0]].signature.clone(),
            victim_tx: "N/A".to_string(),
            value_extracted_lamports: indices.iter().map(|&i| txs[i].priority_fee_lamports).sum(),
            description: format!(
                "Repeat signer: {}... has {} txs in this block",
                &signer[..8.min(signer.len())],
                indices.len(),
            ),
        });
    }

    // --- Pattern 3: Fee disparity (10x+ above average) ---
    let total_fees: u64 = txs.iter().map(|t| t.priority_fee_lamports).sum();
    let avg_fee = if !txs.is_empty() { total_fees / txs.len() as u64 } else { 0 };
    if avg_fee > 0 {
        for tx in txs.iter() {
            if tx.priority_fee_lamports >= avg_fee * 10 && tx.priority_fee_lamports > 100_000 {
                events.push(MevEvent {
                    event_type: MevType::FeeDisparity,
                    extractor_tx: tx.signature.clone(),
                    victim_tx: "all other txs".to_string(),
                    value_extracted_lamports: tx.priority_fee_lamports,
                    description: format!(
                        "Fee disparity: {}... paid {} (avg: {}) — {}x above average",
                        &tx.signer[..8.min(tx.signer.len())],
                        tx.priority_fee_lamports,
                        avg_fee,
                        tx.priority_fee_lamports / avg_fee,
                    ),
                });
            }
        }
    }

    // --- Pattern 4: Coordinated buys (different signers, same pool, sequential) ---
    for i in 0..txs.len() {
        let mut cluster_end = i;
        let i_writes: HashSet<&str> = txs[i].write_accounts.iter().map(|s| s.as_str()).collect();
        
        for j in (i + 1)..txs.len().min(i + 6) {
            let j_writes: HashSet<&str> = txs[j].write_accounts.iter().map(|s| s.as_str()).collect();
            let overlap: Vec<&&str> = i_writes.intersection(&j_writes).collect();
            if !overlap.is_empty() && txs[i].signer != txs[j].signer {
                cluster_end = j;
            } else {
                break;
            }
        }
        
        let cluster_size = cluster_end - i + 1;
        if cluster_size >= 3 {
            // 3+ different signers hitting same accounts in sequence
            let unique_signers: HashSet<&str> = txs[i..=cluster_end].iter().map(|t| t.signer.as_str()).collect();
            if unique_signers.len() >= 3 {
                let total_value: u64 = txs[i..=cluster_end].iter().map(|t| t.priority_fee_lamports).sum();
                events.push(MevEvent {
                    event_type: MevType::CoordinatedBuy,
                    extractor_tx: txs[i].signature.clone(),
                    victim_tx: "retail".to_string(),
                    value_extracted_lamports: total_value,
                    description: format!(
                        "Coordinated: {} different wallets hit same pool in {} sequential txs",
                        unique_signers.len(),
                        cluster_size,
                    ),
                });
            }
        }
    }

    events
}


/// Scan multiple recent blocks and return the most interesting one.
/// "Interesting" = most DEX transactions + highest total priority fees + detected MEV.
///
/// # Arguments
/// * `api_key` - Your Helius API key
/// * `dex_only` - If true, only return Raydium/Pump.fun transactions
/// * `max_txs` - Maximum number of transactions to return
/// * `blocks_to_scan` - Number of recent blocks to check (default 20)
pub async fn fetch_live_transactions(
    api_key: &str,
    dex_only: bool,
    max_txs: usize,
) -> Result<(Vec<SimTransaction>, u64), Box<dyn std::error::Error>> {
    fetch_best_block(api_key, dex_only, max_txs, 20).await
}

/// Scan `blocks_to_scan` recent blocks and pick the most interesting one.
pub async fn fetch_best_block(
    api_key: &str,
    dex_only: bool,
    max_txs: usize,
    blocks_to_scan: usize,
) -> Result<(Vec<SimTransaction>, u64), Box<dyn std::error::Error>> {
    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
    let client = Client::new();

    println!("  Fetching latest slot...");
    let latest_slot = get_latest_slot(&client, &rpc_url).await?;
    println!("  Latest slot: {}", latest_slot);
    println!("  Scanning up to {} recent blocks for interesting activity...", blocks_to_scan);

    let mut best_txs: Vec<SimTransaction> = Vec::new();
    let mut best_slot: u64 = 0;
    let mut best_score: u64 = 0;
    let mut blocks_found = 0;

    for offset in 0..(blocks_to_scan as u64 * 2) {
        // Stop if we've found enough blocks
        if blocks_found >= blocks_to_scan {
            break;
        }

        let try_slot = latest_slot - offset;
        match get_block(&client, &rpc_url, try_slot).await {
            Ok(block) => {
                blocks_found += 1;
                let mut txs = parse_block_transactions(&block, dex_only);
                
                if txs.is_empty() {
                    continue;
                }

                // Detect MEV patterns
                let mev_events = detect_mev_patterns(&mut txs);

                // Score this block: more DEX txs + higher fees + more MEV = more interesting
                let tx_count = txs.len() as u64;
                let total_fees: u64 = txs.iter().map(|t| t.priority_fee_lamports).sum();
                let mev_count = mev_events.len() as u64;
                let score = tx_count * 1000 + total_fees / 1000 + mev_count * 5000;

                if score > best_score {
                    best_score = score;
                    best_txs = txs;
                    best_slot = try_slot;
                    println!(
                        "    Slot {} — {} txs, {} total fees, {} MEV patterns {}",
                        try_slot,
                        tx_count,
                        total_fees,
                        mev_count,
                        "(new best)".to_string().green()
                    );
                } else {
                    println!(
                        "    Slot {} — {} txs, {} total fees",
                        try_slot, tx_count, total_fees
                    );
                }
            }
            Err(_) => continue,
        }
    }

    if best_txs.is_empty() {
        return Err("No transactions found in any scanned block".into());
    }

    // Re-run detection on best block (since we need to return the txs with bundle flags)
    detect_mev_patterns(&mut best_txs);

    let total_in_block = best_txs.len();
    best_txs.truncate(max_txs);

    println!(
        "\n  Selected slot {} — {} DEX transactions (showing {})",
        best_slot, total_in_block, best_txs.len()
    );

    Ok((best_txs, best_slot))
}