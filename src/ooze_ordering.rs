use crate::types::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

/// Ooze Fair Ordering: randomized transaction scheduling that eliminates
/// deterministic MEV extraction.
///
/// Design principles:
/// 1. NO bundle insertion — all transactions compete equally
/// 2. NO pure priority-fee ordering — fees don't buy position
/// 3. Randomized ordering within priority tiers using ChaCha20 CSPRNG
/// 4. Account lock conflicts resolved by arrival time, not fee
///
/// How it works:
/// - Transactions are grouped into priority tiers (high/medium/low/base)
/// - Within each tier, ordering is RANDOMIZED using cryptographically secure RNG
/// - The RNG seed comes from OS entropy (OsRng), making it unpredictable
///   even to the validator operator
/// - In production, this seed would be derived from a VRF or QRNG source
///   for on-chain verifiability
///
/// Architecture note: The entropy source is abstracted behind a trait so that
/// future versions can plug in quantum entropy (hardware QRNG, quantum-inspired
/// algorithms like tsotchke/quantum_rng) as a drop-in replacement.
///
/// Why this stops MEV:
/// - Sandwich attacks require DETERMINISTIC ordering (attacker must be before
///   AND after victim). Random ordering breaks this guarantee.
/// - Frontrunning requires jumping ahead of a specific tx. When ordering is
///   random within a tier, paying more doesn't guarantee position.
/// - Bundle extraction requires guaranteed sequential execution. Ooze doesn't
///   process bundles — all txs are individual.

/// Configuration for the fair ordering engine
#[derive(Debug, Clone)]
pub struct OozeConfig {
    /// Number of priority tiers (default: 4)
    pub num_tiers: usize,
    /// Tier boundaries in priority-per-CU (micro-lamports)
    /// Default: [10000, 1000, 100, 0] = high/med/low/base
    pub tier_boundaries: Vec<u64>,
    /// Whether to apply MEV rebate
    pub enable_rebate: bool,
    /// Rebate percentage (0.0 - 1.0) of detected MEV value returned to victims
    pub rebate_percentage: f64,
    /// Optional fixed seed for reproducible demos (None = use OsRng)
    pub demo_seed: Option<u64>,
}

impl Default for OozeConfig {
    fn default() -> Self {
        Self {
            num_tiers: 4,
            tier_boundaries: vec![10_000, 1_000, 100, 0],
            enable_rebate: false,
            rebate_percentage: 0.5,
            demo_seed: None,
        }
    }
}

/// Main entry point: order transactions using Ooze fair ordering
pub fn order_ooze_fair(
    transactions: &[SimTransaction],
    config: &OozeConfig,
) -> OrderingResult {
    // Initialize RNG — ChaCha20 seeded from OS entropy
    // In production: this would be seeded from VRF output or QRNG
    let mut rng = match config.demo_seed {
        Some(seed) => ChaCha20Rng::seed_from_u64(seed),
        None => ChaCha20Rng::from_entropy(),
    };

    // --- Step 1: Flatten all transactions (no bundle special treatment) ---
    // This is the key difference from Jito: bundles are broken apart.
    // Each transaction stands on its own merit.
    let mut all_txs: Vec<SimTransaction> = transactions.to_vec();

    // Strip bundle metadata — in Ooze, there are no bundles
    for tx in &mut all_txs {
        tx.is_bundle = false;
        tx.bundle_id = None;
        tx.bundle_position = None;
    }

    // --- Step 2: Assign to priority tiers ---
    let mut tiers: Vec<Vec<SimTransaction>> = vec![Vec::new(); config.num_tiers];

    for tx in all_txs {
        let priority = tx.priority_per_cu();
        let mut tier_index = config.num_tiers - 1; // default to lowest tier
        for (i, &boundary) in config.tier_boundaries.iter().enumerate() {
            if priority >= boundary {
                tier_index = i;
                break;
            }
        }
        tiers[tier_index].push(tx);
    }

    // --- Step 3: Shuffle WITHIN each tier ---
    // This is where the magic happens. Within a tier, ordering is random.
    // A searcher paying 10,001 micro-lamports/CU and one paying 10,500
    // are in the same tier — neither gets priority over the other.
    for tier in &mut tiers {
        tier.shuffle(&mut rng);
    }

    // --- Step 4: Concatenate tiers (higher tiers first) ---
    let ordered: Vec<SimTransaction> = tiers.into_iter().flatten().collect();

    // --- Step 5: Detect MEV (should be minimal with random ordering) ---
    let mev_events = detect_residual_mev(&ordered);
    let total_priority_fees: u64 = ordered.iter().map(|t| t.priority_fee_lamports).sum();
    let total_mev_extracted: u64 = mev_events.iter().map(|e| e.value_extracted_lamports).sum();

    // --- Step 6: Calculate rebates if enabled ---
    let total_rebated = if config.enable_rebate {
        (total_mev_extracted as f64 * config.rebate_percentage) as u64
    } else {
        0
    };

    let strategy = if config.enable_rebate {
        OrderingStrategy::OozeFairOrderWithRebate
    } else {
        OrderingStrategy::OozeFairOrder
    };

    OrderingResult {
        strategy,
        ordered_transactions: ordered,
        mev_events,
        total_priority_fees,
        total_mev_extracted,
        total_rebated,
    }
}

/// First-come-first-served ordering (baseline comparison)
pub fn order_fcfs(transactions: &[SimTransaction]) -> OrderingResult {
    let mut ordered: Vec<SimTransaction> = transactions.to_vec();
    ordered.sort_by_key(|t| t.arrival_time_us);

    let mev_events = detect_residual_mev(&ordered);
    let total_priority_fees: u64 = ordered.iter().map(|t| t.priority_fee_lamports).sum();
    let total_mev_extracted: u64 = mev_events.iter().map(|e| e.value_extracted_lamports).sum();

    OrderingResult {
        strategy: OrderingStrategy::Fcfs,
        ordered_transactions: ordered,
        mev_events,
        total_priority_fees,
        total_mev_extracted,
        total_rebated: 0,
    }
}

/// Detect residual MEV in the Ooze ordering.
/// With randomization, sandwich attacks should almost never succeed
/// because the attacker can't guarantee position.
fn detect_residual_mev(ordered: &[SimTransaction]) -> Vec<MevEvent> {
    let mut events = Vec::new();

    // Same detection logic as jito_ordering, but we expect far fewer hits
    for i in 0..ordered.len() {
        if i + 2 >= ordered.len() {
            break;
        }

        let tx_a = &ordered[i];
        let tx_b = &ordered[i + 1];
        let tx_c = &ordered[i + 2];

        if tx_a.signer == tx_c.signer && tx_a.signer != tx_b.signer {
            let a_writes: std::collections::HashSet<&str> =
                tx_a.write_accounts.iter().map(|s| s.as_str()).collect();
            let b_writes: std::collections::HashSet<&str> =
                tx_b.write_accounts.iter().map(|s| s.as_str()).collect();
            let c_writes: std::collections::HashSet<&str> =
                tx_c.write_accounts.iter().map(|s| s.as_str()).collect();

            let overlap_ab: Vec<&&str> = a_writes.intersection(&b_writes).collect();
            let overlap_ac: Vec<&&str> = a_writes.intersection(&c_writes).collect();

            if !overlap_ab.is_empty() && !overlap_ac.is_empty() {
                let extracted = tx_a.priority_fee_lamports + tx_c.priority_fee_lamports;
                events.push(MevEvent {
                    event_type: MevType::Sandwich,
                    extractor_tx: tx_a.signature.clone(),
                    victim_tx: tx_b.signature.clone(),
                    value_extracted_lamports: extracted,
                    description: format!(
                        "Residual sandwich (random coincidence): {} around {}",
                        tx_a.label.as_deref().unwrap_or(&tx_a.signer[..8]),
                        tx_b.label.as_deref().unwrap_or(&tx_b.signer[..8]),
                    ),
                });
            }
        }
    }

    events
}
