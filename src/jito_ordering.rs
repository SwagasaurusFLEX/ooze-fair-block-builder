use crate::types::*;

/// Jito-style ordering: priority fee auction + bundle insertion.
///
/// This replicates the behavior of Agave's Central Scheduler with Jito's
/// BundleStage layered on top:
///
/// 1. Bundles from the block engine get inserted first (highest tip wins)
/// 2. Remaining transactions sorted by priority_fee / compute_units (descending)
/// 3. Account lock conflicts are resolved by priority (higher fee wins the slot)
///
/// Reference: agave/core/src/banking_stage/transaction_scheduler/receive_and_buffer.rs
/// The `calculate_priority_and_cost` function computes priority as:
///   priority = (fee * 1_000_000) / compute_units
///
/// This is the system that enables MEV extraction:
/// - Searchers pay high priority fees to guarantee ordering
/// - Bundles guarantee atomic, sequential execution
/// - A sandwich attack = [attacker_buy, victim_buy, attacker_sell] bundle
pub fn order_jito_auction(transactions: &[SimTransaction]) -> OrderingResult {
    let mut ordered = Vec::new();
    let mut non_bundle_txs: Vec<SimTransaction> = Vec::new();

    // --- Step 1: Group bundles ---
    // In Jito's system, the Block Engine runs an auction every ~200ms.
    // Winning bundles are forwarded to the validator's BundleStage.
    // BundleStage inserts them at the front of the block.
    let mut bundles: std::collections::HashMap<String, Vec<SimTransaction>> =
        std::collections::HashMap::new();

    for tx in transactions {
        if tx.is_bundle {
            if let Some(ref bundle_id) = tx.bundle_id {
                bundles.entry(bundle_id.clone()).or_default().push(tx.clone());
            }
        } else {
            non_bundle_txs.push(tx.clone());
        }
    }

    // Sort bundles by total tip (sum of priority fees in the bundle)
    let mut sorted_bundles: Vec<(String, Vec<SimTransaction>)> = bundles.into_iter().collect();
    sorted_bundles.sort_by(|a, b| {
        let tip_a: u64 = a.1.iter().map(|t| t.priority_fee_lamports).sum();
        let tip_b: u64 = b.1.iter().map(|t| t.priority_fee_lamports).sum();
        tip_b.cmp(&tip_a) // highest tip first
    });

    // Insert bundles in order, preserving internal bundle ordering
    for (_bundle_id, mut bundle_txs) in sorted_bundles {
        bundle_txs.sort_by_key(|t| t.bundle_position.unwrap_or(0));
        ordered.extend(bundle_txs);
    }

    // --- Step 2: Sort remaining by priority per CU (descending) ---
    // This matches Agave's central scheduler behavior:
    // "Conflicting transactions will always be processed in priority order"
    non_bundle_txs.sort_by(|a, b| b.priority_per_cu().cmp(&a.priority_per_cu()));
    ordered.extend(non_bundle_txs);

    // --- Step 3: Detect MEV events ---
    let mev_events = detect_mev_events(&ordered);
    let total_priority_fees: u64 = ordered.iter().map(|t| t.priority_fee_lamports).sum();
    let total_mev_extracted: u64 = mev_events.iter().map(|e| e.value_extracted_lamports).sum();

    OrderingResult {
        strategy: OrderingStrategy::JitoAuction,
        ordered_transactions: ordered,
        mev_events,
        total_priority_fees,
        total_mev_extracted,
        total_rebated: 0, // Jito doesn't rebate
    }
}

/// Detect MEV events in an ordered transaction list.
///
/// Looks for patterns:
/// - Sandwich: tx_A (buy token X) -> tx_B (buy token X, higher fee) -> tx_C (sell token X)
///   where A and C share a signer (the attacker) and B is the victim
/// - Frontrun: tx_A writes to same accounts as tx_B but has higher priority fee
///   and arrived AFTER tx_B (paid to jump the queue)
/// - Bundle extraction: bundle that writes to accounts another non-bundle tx reads
fn detect_mev_events(ordered: &[SimTransaction]) -> Vec<MevEvent> {
    let mut events = Vec::new();

    // Simple sandwich detection: look for A-B-C patterns where
    // A and C share a signer, B is a different signer, and all touch same write accounts
    for i in 0..ordered.len() {
        if i + 2 >= ordered.len() {
            break;
        }

        let tx_a = &ordered[i];
        let tx_b = &ordered[i + 1];
        let tx_c = &ordered[i + 2];

        // Check if A and C share a signer (same attacker) and B is different (victim)
        if tx_a.signer == tx_c.signer && tx_a.signer != tx_b.signer {
            // Check if they touch overlapping write accounts (same market/token)
            let a_writes: std::collections::HashSet<&str> =
                tx_a.write_accounts.iter().map(|s| s.as_str()).collect();
            let b_writes: std::collections::HashSet<&str> =
                tx_b.write_accounts.iter().map(|s| s.as_str()).collect();
            let c_writes: std::collections::HashSet<&str> =
                tx_c.write_accounts.iter().map(|s| s.as_str()).collect();

            let overlap_ab: Vec<&&str> = a_writes.intersection(&b_writes).collect();
            let overlap_ac: Vec<&&str> = a_writes.intersection(&c_writes).collect();

            if !overlap_ab.is_empty() && !overlap_ac.is_empty() {
                // Sandwich detected
                let extracted = tx_a.priority_fee_lamports + tx_c.priority_fee_lamports;
                events.push(MevEvent {
                    event_type: MevType::Sandwich,
                    extractor_tx: tx_a.signature.clone(),
                    victim_tx: tx_b.signature.clone(),
                    value_extracted_lamports: extracted,
                    description: format!(
                        "Sandwich on {}: {} buys before, sells after victim {}",
                        overlap_ab.first().map(|s| **s).unwrap_or("unknown"),
                        tx_a.label.as_deref().unwrap_or(&tx_a.signer[..8]),
                        tx_b.label.as_deref().unwrap_or(&tx_b.signer[..8]),
                    ),
                });
            }
        }

        // Check for frontrunning via bundles
        if tx_a.is_bundle && !tx_b.is_bundle {
            let a_writes: std::collections::HashSet<&str> =
                tx_a.write_accounts.iter().map(|s| s.as_str()).collect();
            let b_writes: std::collections::HashSet<&str> =
                tx_b.write_accounts.iter().map(|s| s.as_str()).collect();

            let overlap: Vec<&&str> = a_writes.intersection(&b_writes).collect();
            if !overlap.is_empty() && tx_a.arrival_time_us > tx_b.arrival_time_us {
                // Bundle arrived AFTER the victim but got placed BEFORE — frontrun
                events.push(MevEvent {
                    event_type: MevType::Frontrun,
                    extractor_tx: tx_a.signature.clone(),
                    victim_tx: tx_b.signature.clone(),
                    value_extracted_lamports: tx_a.priority_fee_lamports,
                    description: format!(
                        "Bundle frontrun: {} jumped ahead of {} on {}",
                        tx_a.label.as_deref().unwrap_or(&tx_a.signer[..8]),
                        tx_b.label.as_deref().unwrap_or(&tx_b.signer[..8]),
                        overlap.first().map(|s| **s).unwrap_or("unknown"),
                    ),
                });
            }
        }
    }

    events
}
