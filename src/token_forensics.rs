use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

// ══════════════════════════════════════════════════════════════════════
// Solana Tracker response types
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Clone)]
pub struct TokenOverview {
    pub token: TokenMeta,
    pub pools: Vec<Pool>,
    pub events: Option<PriceEvents>,
    pub risk: Risk,
    #[serde(default)]
    pub buys: u64,
    #[serde(default)]
    pub sells: u64,
    #[serde(default)]
    pub txns: u64,
    #[serde(default)]
    pub holders: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TokenMeta {
    pub name: String,
    pub symbol: String,
    pub mint: String,
    pub decimals: u8,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub image: String,
    pub creation: Option<Creation>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Creation {
    pub creator: String,
    pub created_tx: String,
    pub created_time: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pool {
    #[serde(rename = "poolId")]
    pub pool_id: String,
    pub liquidity: Liquidity,
    pub price: PriceData,
    #[serde(rename = "tokenSupply")]
    pub token_supply: f64,
    #[serde(rename = "marketCap")]
    pub market_cap: MarketCap,
    pub market: String,
    pub txns: Option<PoolTxns>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Liquidity { pub quote: f64, pub usd: f64 }

#[derive(Debug, Deserialize, Clone)]
pub struct PriceData {
    pub quote: Option<f64>,
    pub usd: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MarketCap { pub quote: f64, pub usd: f64 }

#[derive(Debug, Deserialize, Clone)]
pub struct PoolTxns {
    #[serde(default)] pub buys: u64,
    #[serde(default)] pub sells: u64,
    #[serde(default)] pub total: u64,
    #[serde(default)] pub volume: f64,
    #[serde(rename = "volume24h", default)] pub volume_24h: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct PriceEvents {
    #[serde(rename = "1m")] pub m1: Option<PriceChange>,
    #[serde(rename = "5m")] pub m5: Option<PriceChange>,
    #[serde(rename = "15m")] pub m15: Option<PriceChange>,
    #[serde(rename = "30m")] pub m30: Option<PriceChange>,
    #[serde(rename = "1h")] pub h1: Option<PriceChange>,
    #[serde(rename = "6h")] pub h6: Option<PriceChange>,
    #[serde(rename = "24h")] pub h24: Option<PriceChange>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PriceChange {
    #[serde(rename = "priceChangePercentage")]
    pub pct: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Risk {
    pub snipers: Option<RiskGroup>,
    pub bundlers: Option<BundlersGroup>,
    pub top10: Option<f64>,
    pub score: Option<u64>,
    #[serde(default)]
    pub rugged: bool,
    #[serde(default)]
    pub risks: Vec<RiskItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RiskGroup {
    #[serde(default)] pub count: u64,
    #[serde(rename = "totalPercentage", default)] pub total_percentage: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BundlersGroup {
    #[serde(default)] pub count: u64,
    #[serde(rename = "totalBalance", default)] pub total_balance: f64,
    #[serde(rename = "totalPercentage", default)] pub total_percentage: f64,
    #[serde(rename = "totalInitialBalance", default)] pub total_initial_balance: f64,
    #[serde(rename = "totalInitialPercentage", default)] pub total_initial_percentage: f64,
    #[serde(default)] pub wallets: Vec<BundlerWallet>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BundlerWallet {
    pub wallet: String,
    #[serde(default)] pub balance: f64,
    #[serde(rename = "bundleTime", default)] pub bundle_time: u64,
    #[serde(default)] pub percentage: f64,
    #[serde(rename = "initialBalance", default)] pub initial_balance: f64,
    #[serde(rename = "initialPercentage", default)] pub initial_percentage: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RiskItem {
    pub name: String,
    pub description: String,
    pub level: String,
    #[serde(default)] pub score: u64,
}

// ── Top Traders ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct TopTrader {
    pub wallet: String,
    #[serde(default)] pub held: f64,
    #[serde(default)] pub sold: f64,
    #[serde(default)] pub holding: f64,
    #[serde(default)] pub realized: f64,
    #[serde(default)] pub unrealized: f64,
    #[serde(default)] pub total: f64,
    #[serde(default, rename = "total_invested")] pub total_invested: f64,
    pub tx_counts: Option<TxCounts>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TxCounts {
    #[serde(default)] pub buys: u64,
    #[serde(default)] pub sells: u64,
}

// ══════════════════════════════════════════════════════════════════════
// Our forensics report
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CrossRefRow {
    pub wallet: String,
    pub is_bundler: bool,
    pub initial_pct: f64,
    pub bundle_time: u64,
    pub current_held_pct: f64,
    pub tokens_sold: f64,
    pub realized_profit_usd: f64,
    pub total_profit_usd: f64,
    pub buys: u64,
    pub sells: u64,
    pub invested_usd: f64,
    pub sold_pct_of_held: f64,
}

#[derive(Debug, Clone)]
pub struct ForensicsReport {
    pub overview: TokenOverview,
    pub top_traders: Vec<TopTrader>,
    pub primary_pool: Pool,
    pub cross_ref: Vec<CrossRefRow>,
    pub bundler_count: u64,
    pub bundler_total_initial_pct: f64,
    pub bundler_total_current_pct: f64,
    pub bundlers_in_top_profiteers: usize,
    pub top_profiteers_checked: usize,
    pub total_bundler_profit_usd: f64,
    pub total_non_bundler_profit_usd: f64,
    pub total_retail_loss_usd: f64,
    pub age_hours: f64,
}

// ══════════════════════════════════════════════════════════════════════
// Fetching
// ══════════════════════════════════════════════════════════════════════

async fn fetch_overview(
    client: &Client,
    api_key: &str,
    mint: &str,
) -> Result<TokenOverview, Box<dyn std::error::Error>> {
    let url = format!("https://data.solanatracker.io/tokens/{}", mint);
    let raw = client.get(&url).header("x-api-key", api_key).send().await?;
    let status = raw.status();
    let body = raw.text().await?;
    if !status.is_success() {
        return Err(format!("Overview API ({}): {}", status, &body[..200.min(body.len())]).into());
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("Overview parse: {} — {}", e, &body[..300.min(body.len())]).into())
}

async fn fetch_top_traders(
    client: &Client,
    api_key: &str,
    mint: &str,
) -> Result<Vec<TopTrader>, Box<dyn std::error::Error>> {
    let url = format!("https://data.solanatracker.io/top-traders/{}", mint);
    let raw = client.get(&url).header("x-api-key", api_key).send().await?;
    let status = raw.status();
    let body = raw.text().await?;
    if !status.is_success() {
        return Err(format!("Traders API ({}): {}", status, &body[..200.min(body.len())]).into());
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("Traders parse: {} — {}", e, &body[..300.min(body.len())]).into())
}

// ══════════════════════════════════════════════════════════════════════
// Analysis
// ══════════════════════════════════════════════════════════════════════

fn pick_primary_pool(pools: &[Pool]) -> Pool {
    // Pick the pool with highest USD liquidity (main trading venue)
    pools.iter()
        .max_by(|a, b| a.liquidity.usd.partial_cmp(&b.liquidity.usd).unwrap_or(std::cmp::Ordering::Equal))
        .cloned()
        .unwrap_or_else(|| pools[0].clone())
}

fn build_cross_reference(
    overview: &TokenOverview,
    traders: &[TopTrader],
) -> Vec<CrossRefRow> {
    let bundler_map: HashMap<String, &BundlerWallet> = overview.risk.bundlers
        .as_ref()
        .map(|b| b.wallets.iter().map(|w| (w.wallet.clone(), w)).collect())
        .unwrap_or_default();

    let trader_map: HashMap<String, &TopTrader> = traders.iter()
        .map(|t| (t.wallet.clone(), t))
        .collect();

    // Union of all wallets from both sources
    let mut all_wallets: HashSet<String> = HashSet::new();
    for w in bundler_map.keys() { all_wallets.insert(w.clone()); }
    for w in trader_map.keys() { all_wallets.insert(w.clone()); }

    let mut rows: Vec<CrossRefRow> = all_wallets.iter().map(|wallet| {
        let bundler = bundler_map.get(wallet);
        let trader = trader_map.get(wallet);

        let (is_bundler, initial_pct, bundle_time, current_held_pct) = match bundler {
            Some(b) => (true, b.initial_percentage, b.bundle_time, b.percentage),
            None => (false, 0.0, 0u64, 0.0),
        };

        let (tokens_sold, realized, total_profit, buys, sells, invested) = match trader {
            Some(t) => {
                let tx = t.tx_counts.as_ref();
                (t.sold, t.realized, t.total,
                 tx.map(|x| x.buys).unwrap_or(0),
                 tx.map(|x| x.sells).unwrap_or(0),
                 t.total_invested)
            },
            None => (0.0, 0.0, 0.0, 0, 0, 0.0),
        };

        let sold_pct = if let Some(t) = trader {
            if t.held > 0.0 { t.sold / t.held * 100.0 } else { 0.0 }
        } else { 0.0 };

        CrossRefRow {
            wallet: wallet.clone(),
            is_bundler, initial_pct, bundle_time, current_held_pct,
            tokens_sold, realized_profit_usd: realized, total_profit_usd: total_profit,
            buys, sells, invested_usd: invested,
            sold_pct_of_held: sold_pct,
        }
    }).collect();

    // Sort by total profit descending
    rows.sort_by(|a, b| b.total_profit_usd.partial_cmp(&a.total_profit_usd).unwrap_or(std::cmp::Ordering::Equal));
    rows
}

pub async fn analyze_token(
    api_key: &str,
    mint: &str,
) -> Result<ForensicsReport, Box<dyn std::error::Error>> {
    let client = Client::new();

    println!("  Fetching token overview...");
    let overview = fetch_overview(&client, api_key, mint).await?;
    println!("    ✓ {} ({}) — {} pools", overview.token.name, overview.token.symbol, overview.pools.len());

    // Rate limit
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

    println!("  Fetching top traders...");
    let traders = fetch_top_traders(&client, api_key, mint).await?;
    println!("    ✓ {} profitable wallets", traders.len());

    let primary_pool = pick_primary_pool(&overview.pools);
    let cross_ref = build_cross_reference(&overview, &traders);

    // Stats
    let bundler_count = overview.risk.bundlers.as_ref().map(|b| b.count).unwrap_or(0);
    let bundler_total_initial_pct = overview.risk.bundlers.as_ref().map(|b| b.total_initial_percentage).unwrap_or(0.0);
    let bundler_total_current_pct = overview.risk.bundlers.as_ref().map(|b| b.total_percentage).unwrap_or(0.0);

    // How many of top 50 profiteers are bundlers?
    let top_n = 50.min(cross_ref.len());
    let top_slice = &cross_ref[..top_n];
    let bundlers_in_top: usize = top_slice.iter().filter(|r| r.is_bundler && r.total_profit_usd > 0.0).count();

    // Total profits split by bundler vs non-bundler
    let total_bundler_profit: f64 = cross_ref.iter()
        .filter(|r| r.is_bundler && r.total_profit_usd > 0.0)
        .map(|r| r.total_profit_usd)
        .sum();
    let total_non_bundler_profit: f64 = cross_ref.iter()
        .filter(|r| !r.is_bundler && r.total_profit_usd > 0.0)
        .map(|r| r.total_profit_usd)
        .sum();
    let total_retail_loss: f64 = cross_ref.iter()
        .filter(|r| !r.is_bundler && r.total_profit_usd < 0.0)
        .map(|r| r.total_profit_usd.abs())
        .sum();

    // Age
    let now_secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let age_hours = overview.token.creation.as_ref()
        .map(|c| (now_secs.saturating_sub(c.created_time)) as f64 / 3600.0)
        .unwrap_or(0.0);

    Ok(ForensicsReport {
        overview, top_traders: traders, primary_pool, cross_ref,
        bundler_count, bundler_total_initial_pct, bundler_total_current_pct,
        bundlers_in_top_profiteers: bundlers_in_top,
        top_profiteers_checked: top_n,
        total_bundler_profit_usd: total_bundler_profit,
        total_non_bundler_profit_usd: total_non_bundler_profit,
        total_retail_loss_usd: total_retail_loss,
        age_hours,
    })
}