use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a simplified Solana transaction for ordering simulation.
/// In production, this wraps a real `SanitizedTransaction` from the Agave crate.
/// For the hackathon prototype, we model the fields that matter for ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimTransaction {
    /// Unique transaction signature (base58 in production, simplified here)
    pub signature: String,
    /// The wallet that signed this transaction
    pub signer: String,
    /// Accounts this transaction reads from (shared locks)
    pub read_accounts: Vec<String>,
    /// Accounts this transaction writes to (exclusive locks)
    pub write_accounts: Vec<String>,
    /// Base fee in lamports (5000 lamports/sig = protocol minimum)
    pub base_fee_lamports: u64,
    /// Priority fee in lamports (the bidding mechanism)
    pub priority_fee_lamports: u64,
    /// Compute units requested
    pub compute_units: u64,
    /// Timestamp when this tx entered the mempool (unix micros)
    pub arrival_time_us: u64,
    /// Whether this is part of a Jito-style bundle
    pub is_bundle: bool,
    /// Bundle ID if part of a bundle (bundles execute atomically)
    pub bundle_id: Option<String>,
    /// Position within bundle (0 = first)
    pub bundle_position: Option<u32>,
    /// Human-readable label for demo purposes
    pub label: Option<String>,
}

impl SimTransaction {
    /// Total fee = base + priority
    pub fn total_fee(&self) -> u64 {
        self.base_fee_lamports + self.priority_fee_lamports
    }

    /// Priority in micro-lamports per compute unit
    /// This is how Agave's central scheduler calculates priority:
    /// priority = (fee * MICRO_LAMPORTS_PER_LAMPORT) / compute_units
    pub fn priority_per_cu(&self) -> u64 {
        if self.compute_units == 0 {
            return 0;
        }
        // Multiply by 1_000_000 to get micro-lamports, matching Agave's calculation
        (self.priority_fee_lamports * 1_000_000) / self.compute_units
    }
}

impl fmt::Display for SimTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = self.label.as_deref().unwrap_or(&self.signature[..8]);
        write!(
            f,
            "{} | priority_fee: {} | cu: {} | priority/cu: {}",
            label,
            self.priority_fee_lamports,
            self.compute_units,
            self.priority_per_cu()
        )
    }
}

/// The result of ordering a batch of transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderingResult {
    /// Which ordering strategy produced this
    pub strategy: OrderingStrategy,
    /// Transactions in their final execution order
    pub ordered_transactions: Vec<SimTransaction>,
    /// MEV extraction events detected in this ordering
    pub mev_events: Vec<MevEvent>,
    /// Total priority fees collected by the validator
    pub total_priority_fees: u64,
    /// Total value extracted via MEV (sandwiches, frontrunning)
    pub total_mev_extracted: u64,
    /// Value that would be rebated to users under Ooze model
    pub total_rebated: u64,
}

/// Ordering strategies we compare
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderingStrategy {
    /// Agave default: highest priority fee per CU first
    /// With Jito: bundles inserted at top via auction
    JitoAuction,
    /// Ooze: randomized ordering within priority tiers
    /// No bundle insertion, no auction
    OozeFairOrder,
    /// Ooze with MEV rebate: randomized + rebate mechanism
    OozeFairOrderWithRebate,
    /// First-come-first-served (baseline comparison)
    Fcfs,
}

impl fmt::Display for OrderingStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JitoAuction => write!(f, "Jito Auction (status quo)"),
            Self::OozeFairOrder => write!(f, "Ooze Fair Order"),
            Self::OozeFairOrderWithRebate => write!(f, "Ooze Fair Order + Rebate"),
            Self::Fcfs => write!(f, "First-Come-First-Served"),
        }
    }
}

/// Detected MEV extraction event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevEvent {
    pub event_type: MevType,
    /// The transaction doing the extracting
    pub extractor_tx: String,
    /// The transaction being extracted from (the victim)
    pub victim_tx: String,
    /// Estimated value extracted in lamports
    pub value_extracted_lamports: u64,
    /// Description for the demo
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MevType {
    /// Attacker's buy placed before victim's buy
    Frontrun,
    /// Attacker buys before AND sells after victim (sandwich)
    Sandwich,
    /// Bundle guarantees ordering for extraction
    BundleExtraction,
}

impl fmt::Display for MevType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frontrun => write!(f, "Frontrun"),
            Self::Sandwich => write!(f, "Sandwich"),
            Self::BundleExtraction => write!(f, "Bundle Extraction"),
        }
    }
}
