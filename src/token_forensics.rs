use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ══════════════════════════════════════════════════════════════════════
// SOLANA TRACKER API RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenOverview {
    pub token: TokenMeta,
    pub pools: Vec<Pool>,
    pub events: Option<PriceEvents>,
    pub risk: Risk,
    #[serde(default)] pub buys: u64,
    #[serde(default)] pub sells: u64,
    #[serde(default)] pub txns: u64,
    #[serde(default)] pub holders: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenMeta {
    pub name: String,
    pub symbol: String,
    pub mint: String,
    pub decimals: u8,
    pub creation: Option<Creation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Creation {
    pub creator: String,
    pub created_tx: String,
    pub created_time: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Pool {
    #[serde(rename = "poolId")] pub pool_id: String,
    pub liquidity: Liquidity,
    pub price: PriceData,
    #[serde(rename = "tokenSupply")] pub token_supply: f64,
    #[serde(rename = "marketCap")] pub market_cap: MarketCap,
    pub market: String,
    pub txns: Option<PoolTxns>,
    #[serde(rename = "createdAt")] pub created_at: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Liquidity { pub quote: f64, pub usd: f64 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PriceData { pub quote: Option<f64>, pub usd: Option<f64> }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MarketCap { pub quote: f64, pub usd: f64 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PoolTxns {
    #[serde(default)] pub buys: u64,
    #[serde(default)] pub sells: u64,
    #[serde(default)] pub total: u64,
    #[serde(default)] pub volume: f64,
    #[serde(rename = "volume24h", default)] pub volume_24h: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PriceEvents {
    #[serde(rename = "1h")] pub h1: Option<PriceChange>,
    #[serde(rename = "6h")] pub h6: Option<PriceChange>,
    #[serde(rename = "24h")] pub h24: Option<PriceChange>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PriceChange { #[serde(rename = "priceChangePercentage")] pub pct: f64 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Risk {
    pub bundlers: Option<BundlersGroup>,
    pub top10: Option<f64>,
    pub score: Option<u64>,
    #[serde(default)] pub rugged: bool,
    #[serde(default)] pub risks: Vec<RiskItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BundlersGroup {
    #[serde(default)] pub count: u64,
    #[serde(rename = "totalPercentage", default)] pub total_percentage: f64,
    #[serde(rename = "totalInitialPercentage", default)] pub total_initial_percentage: f64,
    #[serde(default)] pub wallets: Vec<BundlerWallet>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BundlerWallet {
    pub wallet: String,
    #[serde(default)] pub balance: f64,
    #[serde(rename = "bundleTime", default)] pub bundle_time: u64,
    #[serde(default)] pub percentage: f64,
    #[serde(rename = "initialBalance", default)] pub initial_balance: f64,
    #[serde(rename = "initialPercentage", default)] pub initial_percentage: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RiskItem {
    pub name: String,
    pub description: String,
    pub level: String,
    #[serde(default)] pub score: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TopTrader {
    pub wallet: String,
    #[serde(default)] pub held: f64,
    #[serde(default)] pub sold: f64,
    #[serde(default)] pub realized: f64,
    #[serde(default)] pub total: f64,
    #[serde(default, rename = "total_invested")] pub total_invested: f64,
    pub tx_counts: Option<TxCounts>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TxCounts {
    #[serde(default)] pub buys: u64,
    #[serde(default)] pub sells: u64,
}

#[derive(Debug, Deserialize)]
pub struct TradesResponse {
    #[serde(default)] pub trades: Vec<RawTrade>,
    #[serde(rename = "nextCursor", default)] pub next_cursor: Option<u64>,
    #[serde(rename = "hasNextPage", default)] pub has_next_page: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RawTrade {
    pub tx: Option<String>,
    pub amount: Option<f64>,
    #[serde(rename = "priceUsd")] pub price_usd: Option<f64>,
    #[serde(rename = "volumeSol")] pub volume_sol: Option<f64>,
    #[serde(rename = "type")] pub trade_type: Option<String>,
    pub wallet: Option<String>,
    pub time: Option<u64>,
    pub program: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChartResponse {
    #[serde(default)] pub oclhv: Vec<Candle>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Candle {
    pub open: f64,
    pub close: f64,
    pub low: f64,
    pub high: f64,
    pub volume: f64,
    pub time: u64,
}

// ══════════════════════════════════════════════════════════════════════
// DOMAIN TYPES
// ══════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Clone)]
pub struct Trade {
    pub signature: String,
    pub wallet: String,
    pub timestamp_ms: u64,
    pub direction: String,
    pub sol_amount: f64,
    pub token_amount: f64,
    pub price_usd: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Cluster {
    pub timestamp_ms: u64,
    pub direction: String,
    pub wallets: Vec<String>,
    pub total_sol: f64,
    pub total_tokens: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct WalletComparison {
    pub wallet: String,
    pub jito_tokens: f64,
    pub ooze_tokens: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct OozeReplay {
    pub trades_in_window: usize,
    pub jito_top_wallet: String,
    pub jito_top_tokens: f64,
    pub jito_top_supply_pct: f64,
    pub ooze_top_wallet: String,
    pub ooze_top_tokens: f64,
    pub ooze_top_supply_pct: f64,
    pub reduction_pct: f64,           // how much fewer tokens top wallet gets under Ooze
    pub price_impact_reduction: f64,  // estimated price impact reduction
    pub wallet_comparisons: Vec<WalletComparison>,  // top 5 wallets, jito vs ooze capture
    pub notes: Vec<String>,
}

/// A dramatic price event detected in the token's OHLCV history.
/// Contains both the candle-level info AND the trade-level analysis.
#[derive(Debug, Serialize, Clone)]
pub struct DramaticEvent {
    // Event identification (from OHLCV)
    pub event_type: String,          // "PUMP" or "DUMP"
    pub severity: String,            // "DRAMATIC" (>=50%), "MAJOR" (>=25%), "MINOR"
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub candle_count: u32,           // 1 or 2
    pub price_start: f64,
    pub price_end: f64,
    pub price_low: f64,
    pub price_high: f64,
    pub price_change_pct: f64,       // signed
    pub abs_magnitude: f64,          // absolute value for ranking
    pub candle_volume_sol: f64,

    // Trade-level analysis (filled after fetching window trades)
    pub trades: Vec<Trade>,
    pub clusters: Vec<Cluster>,
    pub unique_wallets: usize,
    pub coordinated_wallet_count: usize,
    pub coordinated_wallets: Vec<String>,
    pub total_trade_sol: f64,
    pub coordinated_sol: f64,
    pub coordination_pct: f64,       // 0-100, % of trade volume from coordinated wallets
    pub trades_fetched: bool,        // did we manage to get trades for this event?

    // Ooze replay (only for top events)
    pub ooze_replay: Option<OozeReplay>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ForensicsReport {
    pub overview: TokenOverview,
    pub top_traders: Vec<TopTrader>,
    pub primary_pool: Pool,
    pub candles: Vec<Candle>,
    pub events: Vec<DramaticEvent>,
    pub age_hours: f64,
    pub ath_candle: Option<Candle>,
    pub ath_mcap_usd: f64,

    // Summary stats across analyzed events
    pub total_events_detected: usize,
    pub dramatic_events_count: usize,       // >= 50% moves
    pub major_events_count: usize,          // >= 25% moves
    pub events_with_coordination: usize,    // events where coordination_pct > 30
    pub avg_coordination_pct: f64,          // average across all analyzed events
    pub api_calls_used: u32,                // rough count
}

// ══════════════════════════════════════════════════════════════════════
// API FETCHERS
// ══════════════════════════════════════════════════════════════════════

async fn fetch_overview(client: &Client, key: &str, mint: &str) -> Result<TokenOverview, Box<dyn std::error::Error>> {
    let url = format!("https://data.solanatracker.io/tokens/{}", mint);
    let r = client.get(&url).header("x-api-key", key).send().await?;
    let s = r.status();
    let body = r.text().await?;
    if !s.is_success() {
        return Err(format!("Overview ({}): {}", s, &body[..200.min(body.len())]).into());
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("Overview parse: {} — {}", e, &body[..300.min(body.len())]).into())
}

async fn fetch_top_traders(client: &Client, key: &str, mint: &str) -> Result<Vec<TopTrader>, Box<dyn std::error::Error>> {
    let url = format!("https://data.solanatracker.io/top-traders/{}", mint);
    let r = client.get(&url).header("x-api-key", key).send().await?;
    let s = r.status();
    let body = r.text().await?;
    if !s.is_success() {
        return Err(format!("Traders ({}): {}", s, &body[..200.min(body.len())]).into());
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("Traders parse: {} — {}", e, &body[..300.min(body.len())]).into())
}

async fn fetch_chart(
    client: &Client, key: &str, mint: &str,
    interval: &str, from_s: u64, to_s: u64,
) -> Result<Vec<Candle>, Box<dyn std::error::Error>> {
    let url = format!(
        "https://data.solanatracker.io/chart/{}?type={}&time_from={}&time_to={}",
        mint, interval, from_s, to_s
    );
    let r = client.get(&url).header("x-api-key", key).send().await?;
    let s = r.status();
    let body = r.text().await?;
    if !s.is_success() {
        return Err(format!("Chart ({}): {}", s, &body[..200.min(body.len())]).into());
    }
    let resp: ChartResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Chart parse: {} — {}", e, &body[..300.min(body.len())]))?;
    Ok(resp.oclhv)
}

async fn fetch_trades_page(
    client: &Client, key: &str, mint: &str,
    cursor: Option<u64>,
) -> Result<TradesResponse, Box<dyn std::error::Error>> {
    let url = match cursor {
        Some(c) => format!("https://data.solanatracker.io/trades/{}?cursor={}", mint, c),
        None => format!("https://data.solanatracker.io/trades/{}", mint),
    };

    // Retry loop for 429 rate limits
    let max_retries = 3;
    for attempt in 0..max_retries {
        let r = client.get(&url).header("x-api-key", key).send().await?;
        let s = r.status();
        let body = r.text().await?;

        if s.is_success() {
            return serde_json::from_str(&body)
                .map_err(|e| format!("Trades parse: {} — {}", e, &body[..300.min(body.len())]).into());
        }

        // On rate limit, back off exponentially and retry
        if s.as_u16() == 429 && attempt < max_retries - 1 {
            let backoff_ms = 3000 * (attempt + 1) as u64; // 3s, 6s, 9s
            eprintln!("         ⏳ rate limited, backing off {}ms (attempt {}/{})", backoff_ms, attempt + 1, max_retries);
            tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;
            continue;
        }

        return Err(format!("Trades ({}): {}", s, &body[..200.min(body.len())]).into());
    }

    Err("Trades: exhausted retries".into())
}

fn raw_to_trade(t: &RawTrade) -> Option<Trade> {
    let dir = t.trade_type.as_deref()?;
    if dir != "buy" && dir != "sell" { return None; }
    Some(Trade {
        signature: t.tx.clone().unwrap_or_default(),
        wallet: t.wallet.clone().unwrap_or_default(),
        timestamp_ms: t.time.unwrap_or(0),
        direction: dir.to_string(),
        sol_amount: t.volume_sol.unwrap_or(0.0),
        token_amount: t.amount.unwrap_or(0.0),
        price_usd: t.price_usd.unwrap_or(0.0),
    })
}

// ══════════════════════════════════════════════════════════════════════
// EVENT DETECTION FROM CANDLES
// ══════════════════════════════════════════════════════════════════════
//
// Three event types:
// 1. Spike events — single candle with dramatic wick movement
// 2. 2-candle events — dramatic move across 2 consecutive candles
// 3. Trend events — cumulative move over 5-30 min windows (catches slow bleeds)
//
// Pumps and dumps are detected separately so asymmetric math doesn't
// let pumps eclipse dumps in the rankings.

const DRAMATIC_THRESHOLD: f64 = 30.0;   // lowered from 50 per user feedback
const MAJOR_THRESHOLD: f64 = 15.0;       // lowered from 25
const MIN_CANDLE_VOLUME: f64 = 0.1;
const TOP_PUMPS: usize = 3;
const TOP_DUMPS: usize = 3;
const TREND_MAX_CANDLES: usize = 30;     // up to 30-minute trend window

fn classify_severity(abs: f64) -> &'static str {
    if abs >= DRAMATIC_THRESHOLD { "DRAMATIC" }
    else if abs >= MAJOR_THRESHOLD { "MAJOR" }
    else { "MINOR" }
}

fn make_event(
    event_type: &str, start_ms: u64, end_ms: u64, candle_count: u32,
    p_start: f64, p_end: f64, p_low: f64, p_high: f64,
    pct: f64, vol_sol: f64,
) -> DramaticEvent {
    let abs = pct.abs();
    let signed = if event_type == "DUMP" { -abs } else { abs };
    DramaticEvent {
        event_type: event_type.to_string(),
        severity: classify_severity(abs).to_string(),
        start_time_ms: start_ms,
        end_time_ms: end_ms,
        candle_count,
        price_start: p_start,
        price_end: p_end,
        price_low: p_low,
        price_high: p_high,
        price_change_pct: signed,
        abs_magnitude: abs,
        candle_volume_sol: vol_sol,
        trades: Vec::new(),
        clusters: Vec::new(),
        unique_wallets: 0,
        coordinated_wallet_count: 0,
        coordinated_wallets: Vec::new(),
        total_trade_sol: 0.0,
        coordinated_sol: 0.0,
        coordination_pct: 0.0,
        trades_fetched: false,
        ooze_replay: None,
    }
}

fn detect_events(candles: &[Candle]) -> Vec<DramaticEvent> {
    if candles.is_empty() { return Vec::new(); }

    let mut pumps: Vec<DramaticEvent> = Vec::new();
    let mut dumps: Vec<DramaticEvent> = Vec::new();

    // ── Single-candle pumps and dumps (wick extremes) ──
    for c in candles {
        if c.open <= 0.0 || c.volume < MIN_CANDLE_VOLUME { continue; }
        let up_pct = (c.high - c.open) / c.open * 100.0;
        let down_pct = (c.open - c.low) / c.open * 100.0;
        let start_ms = c.time * 1000;
        let end_ms = (c.time + 60) * 1000;

        if up_pct >= MAJOR_THRESHOLD {
            pumps.push(make_event("PUMP", start_ms, end_ms, 1,
                c.open, c.high, c.low, c.high, up_pct, c.volume));
        }
        if down_pct >= MAJOR_THRESHOLD {
            dumps.push(make_event("DUMP", start_ms, end_ms, 1,
                c.open, c.low, c.low, c.high, down_pct, c.volume));
        }
    }

    // ── 2-candle pumps and dumps ──
    for i in 0..candles.len().saturating_sub(1) {
        let a = &candles[i];
        let b = &candles[i + 1];
        if a.open <= 0.0 { continue; }
        if a.volume + b.volume < MIN_CANDLE_VOLUME { continue; }

        let combined_high = a.high.max(b.high);
        let combined_low = a.low.min(b.low);
        let up_pct = (combined_high - a.open) / a.open * 100.0;
        let down_pct = (a.open - combined_low) / a.open * 100.0;
        let start_ms = a.time * 1000;
        let end_ms = (b.time + 60) * 1000;
        let vol = a.volume + b.volume;

        if up_pct >= MAJOR_THRESHOLD {
            pumps.push(make_event("PUMP", start_ms, end_ms, 2,
                a.open, combined_high, combined_low, combined_high, up_pct, vol));
        }
        if down_pct >= MAJOR_THRESHOLD {
            dumps.push(make_event("DUMP", start_ms, end_ms, 2,
                a.open, combined_low, combined_low, combined_high, down_pct, vol));
        }
    }

    // ── Trend events: 5–30 min rolling windows catching slow bleeds ──
    // For each starting candle, scan forward until cumulative move hits threshold
    // or we exceed the max window.
    for start_idx in 0..candles.len() {
        let start = &candles[start_idx];
        if start.open <= 0.0 { continue; }
        let mut best_up = (0.0_f64, start_idx, start_idx);    // (pct, end_idx, vol_accumulator_idx)
        let mut best_down = (0.0_f64, start_idx, start_idx);
        let max_end = (start_idx + TREND_MAX_CANDLES).min(candles.len() - 1);

        for end_idx in (start_idx + 4)..=max_end {
            let window_high = candles[start_idx..=end_idx].iter()
                .map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
            let window_low = candles[start_idx..=end_idx].iter()
                .map(|c| c.low).fold(f64::INFINITY, f64::min);
            let up_pct = (window_high - start.open) / start.open * 100.0;
            let down_pct = (start.open - window_low) / start.open * 100.0;
            if up_pct > best_up.0 { best_up = (up_pct, end_idx, end_idx); }
            if down_pct > best_down.0 { best_down = (down_pct, end_idx, end_idx); }
        }

        if best_up.0 >= DRAMATIC_THRESHOLD {
            let end = &candles[best_up.1];
            let vol: f64 = candles[start_idx..=best_up.1].iter().map(|c| c.volume).sum();
            let window_high = candles[start_idx..=best_up.1].iter()
                .map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
            let window_low = candles[start_idx..=best_up.1].iter()
                .map(|c| c.low).fold(f64::INFINITY, f64::min);
            pumps.push(make_event("PUMP", start.time * 1000, (end.time + 60) * 1000,
                (best_up.1 - start_idx + 1) as u32,
                start.open, window_high, window_low, window_high, best_up.0, vol));
        }
        if best_down.0 >= DRAMATIC_THRESHOLD {
            let end = &candles[best_down.1];
            let vol: f64 = candles[start_idx..=best_down.1].iter().map(|c| c.volume).sum();
            let window_high = candles[start_idx..=best_down.1].iter()
                .map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
            let window_low = candles[start_idx..=best_down.1].iter()
                .map(|c| c.low).fold(f64::INFINITY, f64::min);
            dumps.push(make_event("DUMP", start.time * 1000, (end.time + 60) * 1000,
                (best_down.1 - start_idx + 1) as u32,
                start.open, window_low, window_low, window_high, best_down.0, vol));
        }
    }

    // Sort each by magnitude
    pumps.sort_by(|a, b| b.abs_magnitude.partial_cmp(&a.abs_magnitude).unwrap_or(std::cmp::Ordering::Equal));
    dumps.sort_by(|a, b| b.abs_magnitude.partial_cmp(&a.abs_magnitude).unwrap_or(std::cmp::Ordering::Equal));

    // Dedup within each direction by time-overlap, keeping highest-magnitude
    fn dedup_overlapping(events: Vec<DramaticEvent>) -> Vec<DramaticEvent> {
        let mut kept: Vec<DramaticEvent> = Vec::new();
        for e in events {
            let overlaps = kept.iter().any(|k| {
                e.start_time_ms < k.end_time_ms && e.end_time_ms > k.start_time_ms
            });
            if !overlaps { kept.push(e); }
        }
        kept
    }

    let pumps = dedup_overlapping(pumps);
    let dumps = dedup_overlapping(dumps);

    // Take top N of each
    let mut result: Vec<DramaticEvent> = pumps.into_iter().take(TOP_PUMPS).collect();
    result.extend(dumps.into_iter().take(TOP_DUMPS));

    // If we have nothing (very calm token), add the biggest candles even if below threshold
    if result.is_empty() {
        // Find biggest up and down moves regardless of threshold
        let mut biggest_up: Option<DramaticEvent> = None;
        let mut biggest_down: Option<DramaticEvent> = None;
        for c in candles {
            if c.open <= 0.0 { continue; }
            let up = (c.high - c.open) / c.open * 100.0;
            let down = (c.open - c.low) / c.open * 100.0;
            if biggest_up.as_ref().map(|e| up > e.abs_magnitude).unwrap_or(up > 0.0) {
                biggest_up = Some(make_event("PUMP", c.time * 1000, (c.time + 60) * 1000, 1,
                    c.open, c.high, c.low, c.high, up, c.volume));
            }
            if biggest_down.as_ref().map(|e| down > e.abs_magnitude).unwrap_or(down > 0.0) {
                biggest_down = Some(make_event("DUMP", c.time * 1000, (c.time + 60) * 1000, 1,
                    c.open, c.low, c.low, c.high, down, c.volume));
            }
        }
        if let Some(mut e) = biggest_up { e.severity = "MINOR".into(); result.push(e); }
        if let Some(mut e) = biggest_down { e.severity = "MINOR".into(); result.push(e); }
    }

    // Final chronological sort for display
    result.sort_by_key(|e| e.start_time_ms);

    result
}

// ══════════════════════════════════════════════════════════════════════
// FETCH TRADES FOR A SPECIFIC EVENT WINDOW
// ══════════════════════════════════════════════════════════════════════
//
// CRITICAL: Solana Tracker's /trades endpoint cursor is a unix timestamp
// in MILLISECONDS. By passing cursor = event_end_time_ms, we jump DIRECTLY
// to trades at that moment — no need to paginate through thousands of newer trades.

const TRADE_FETCH_BUFFER_MS: u64 = 450_000; // 7.5 minutes on each side = 15 min window

async fn fetch_trades_for_window(
    client: &Client, key: &str, mint: &str,
    start_time_ms: u64, end_time_ms: u64,
) -> Result<(Vec<Trade>, u32), Box<dyn std::error::Error>> {
    let window_start = start_time_ms.saturating_sub(TRADE_FETCH_BUFFER_MS);
    let window_end = end_time_ms + TRADE_FETCH_BUFFER_MS;

    // Cursor is a millisecond timestamp — start at window_end to jump directly there.
    let mut collected: Vec<Trade> = Vec::new();
    let mut cursor: Option<u64> = Some(window_end);
    let mut calls = 0u32;
    let max_pages = 10;

    for _ in 0..max_pages {
        let resp = fetch_trades_page(client, key, mint, cursor).await?;
        calls += 1;

        if resp.trades.is_empty() { break; }

        // Find oldest trade in this page (milliseconds)
        let oldest_in_page = resp.trades.iter().filter_map(|t| t.time).min().unwrap_or(0);

        // Collect matching trades
        for raw in &resp.trades {
            if let Some(t) = raw_to_trade(raw) {
                if t.timestamp_ms >= window_start && t.timestamp_ms <= window_end {
                    collected.push(t);
                }
            }
        }

        // If we've passed the window start, stop
        if oldest_in_page < window_start {
            break;
        }

        if !resp.has_next_page {
            break;
        }
        cursor = resp.next_cursor;

        // Rate limit: free tier is 1 rps — use 1500ms to be safe
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    }

    collected.sort_by_key(|t| t.timestamp_ms);

    Ok((collected, calls))
}

// ══════════════════════════════════════════════════════════════════════
// CLUSTER DETECTION WITHIN A SINGLE EVENT WINDOW
// ══════════════════════════════════════════════════════════════════════

const CLUSTER_WINDOW_MS: u64 = 2_000;
const CLUSTER_MIN_WALLETS: usize = 3;

fn detect_clusters_in_trades(trades: &[Trade]) -> Vec<Cluster> {
    if trades.len() < CLUSTER_MIN_WALLETS { return Vec::new(); }

    let mut sorted: Vec<&Trade> = trades.iter().collect();
    sorted.sort_by_key(|t| t.timestamp_ms);

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut i = 0;

    while i < sorted.len() {
        let window_start = sorted[i].timestamp_ms;
        let window_end = window_start + CLUSTER_WINDOW_MS;

        let mut j = i;
        let mut buys: Vec<&Trade> = Vec::new();
        let mut sells: Vec<&Trade> = Vec::new();
        while j < sorted.len() && sorted[j].timestamp_ms <= window_end {
            if sorted[j].direction == "buy" { buys.push(sorted[j]); }
            else { sells.push(sorted[j]); }
            j += 1;
        }

        for (dir_label, trades_in_dir) in [("buy", &buys), ("sell", &sells)] {
            let wallets: HashSet<String> = trades_in_dir.iter().map(|t| t.wallet.clone()).collect();
            if wallets.len() >= CLUSTER_MIN_WALLETS {
                let total_sol: f64 = trades_in_dir.iter().map(|t| t.sol_amount).sum();
                let total_tokens: f64 = trades_in_dir.iter().map(|t| t.token_amount).sum();
                clusters.push(Cluster {
                    timestamp_ms: window_start,
                    direction: dir_label.to_string(),
                    wallets: wallets.into_iter().collect(),
                    total_sol,
                    total_tokens,
                });
            }
        }

        i = j.max(i + 1);
    }

    clusters.sort_by_key(|c| c.timestamp_ms);
    clusters
}

fn analyze_event_coordination(event: &mut DramaticEvent) {
    event.clusters = detect_clusters_in_trades(&event.trades);

    let unique: HashSet<&str> = event.trades.iter().map(|t| t.wallet.as_str()).collect();
    event.unique_wallets = unique.len();

    // Coordinated wallets = union of all wallets in clusters
    let mut coord_set: HashSet<String> = HashSet::new();
    for cl in &event.clusters {
        for w in &cl.wallets {
            coord_set.insert(w.clone());
        }
    }
    event.coordinated_wallet_count = coord_set.len();

    event.total_trade_sol = event.trades.iter().map(|t| t.sol_amount).sum();
    event.coordinated_sol = event.trades.iter()
        .filter(|t| coord_set.contains(&t.wallet))
        .map(|t| t.sol_amount)
        .sum();

    event.coordination_pct = if event.total_trade_sol > 0.0 {
        (event.coordinated_sol / event.total_trade_sol) * 100.0
    } else { 0.0 };

    event.coordinated_wallets = coord_set.into_iter().collect();
}

// ══════════════════════════════════════════════════════════════════════
// OOZE REPLAY FOR AN EVENT WINDOW
// ══════════════════════════════════════════════════════════════════════

const TOTAL_SUPPLY: f64 = 1_000_000_000.0;

fn ooze_replay(event: &DramaticEvent) -> OozeReplay {
    if event.trades.is_empty() {
        return OozeReplay {
            trades_in_window: 0,
            jito_top_wallet: "—".into(), jito_top_tokens: 0.0, jito_top_supply_pct: 0.0,
            ooze_top_wallet: "—".into(), ooze_top_tokens: 0.0, ooze_top_supply_pct: 0.0,
            reduction_pct: 0.0, price_impact_reduction: 0.0,
            wallet_comparisons: vec![],
            notes: vec!["No trades in window".into()],
        };
    }

    // Only look at the dominant direction for the event
    let target_dir = if event.event_type == "PUMP" { "buy" } else { "sell" };

    // --- Actual (Jito) accumulation per wallet in target direction ---
    let mut jito_accum: HashMap<String, f64> = HashMap::new();
    for t in &event.trades {
        if t.direction == target_dir {
            *jito_accum.entry(t.wallet.clone()).or_insert(0.0) += t.token_amount;
        }
    }
    let (jito_wallet, jito_tokens) = jito_accum.iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(w, t)| (w.clone(), *t))
        .unwrap_or(("—".into(), 0.0));

    // --- Ooze modeled: probabilistic re-ordering within each 2-second window ---
    //
    // Under Jito today, bundles land atomically — the bundling wallet captures
    // the full intended token amount at the intended slippage.
    //
    // Under Ooze, when multiple wallets fire within the same ~2-second window,
    // they all compete for the same liquidity at randomized ordering. A wallet
    // that WOULD have gone first now has a 1/n chance of going first, 1/n of
    // going second, etc. Later positions in a PUMP pay worse prices (less slippage
    // headroom) — so we apply a cumulative dilution where wallets in positions
    // 2..n capture progressively fewer tokens for the same SOL.
    //
    // For each trade, compute its expected token capture under Ooze:
    //   - Find all trades in same 2s window, same direction
    //   - If n = 1, no other wallets: capture full amount
    //   - If n > 1, the wallet's expected token capture is reduced by
    //     (1 - CUMULATIVE_SLIPPAGE_PENALTY * (avg_position - 1) / (n - 1))
    //   - Averaged over positions 1..n, avg_position = (n+1)/2
    //   - So effective_multiplier = 1 - CSP * ((n-1)/2) / (n-1) = 1 - CSP/2
    //   - This means: any cluster of 2+ wallets caps the winner at ~(1 - CSP/2) = 75%
    //   - Bigger clusters: the FIRST-position capture is even more rare (1/n chance),
    //     so we further weight by 1/n for "full-capture probability".
    //
    // Simplified: effective_tokens_ooze = full_tokens * (1/n + (1 - 1/n) * (1 - CSP))
    //   = full_tokens * (1/n * 1 + (n-1)/n * (1 - CSP))
    //
    // With CSP = 0.50 (late positions lose half their tokens on avg):
    //   n=1: 1.0x    (no dilution)
    //   n=2: 0.75x   (50% chance of full + 50% chance of half)
    //   n=3: 0.666x
    //   n=5: 0.60x
    //   n=10: 0.55x

    const BLOCK_WINDOW_MS: u64 = 2_000;
    const CUMULATIVE_SLIPPAGE_PENALTY: f64 = 0.50;

    let mut sorted: Vec<&Trade> = event.trades.iter().filter(|t| t.direction == target_dir).collect();
    sorted.sort_by_key(|t| t.timestamp_ms);

    let mut ooze_accum: HashMap<String, f64> = HashMap::new();
    for (idx, t) in sorted.iter().enumerate() {
        let window_lo = t.timestamp_ms.saturating_sub(BLOCK_WINDOW_MS / 2);
        let window_hi = t.timestamp_ms + BLOCK_WINDOW_MS / 2;
        // Count unique OTHER wallets in same block window (excluding self — a wallet
        // firing many of its own trades in succession shouldn't compete with itself)
        let self_wallet = t.wallet.as_str();
        let others: HashSet<&str> = sorted[idx.saturating_sub(50)..]
            .iter()
            .take(100)
            .filter(|o| o.timestamp_ms >= window_lo && o.timestamp_ms <= window_hi)
            .map(|o| o.wallet.as_str())
            .filter(|w| *w != self_wallet)
            .collect();
        let n = (others.len() + 1) as f64; // +1 to include self
        let full_capture_prob = 1.0 / n;
        let effective_multiplier = full_capture_prob * 1.0
            + (1.0 - full_capture_prob) * (1.0 - CUMULATIVE_SLIPPAGE_PENALTY);
        *ooze_accum.entry(t.wallet.clone()).or_insert(0.0) += t.token_amount * effective_multiplier;
    }

    let (ooze_wallet, ooze_tokens) = ooze_accum.iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(w, t)| (w.clone(), *t))
        .unwrap_or(("—".into(), 0.0));

    let reduction = if jito_tokens > 0.0 {
        (1.0 - ooze_tokens / jito_tokens) * 100.0
    } else { 0.0 };

    // Build top-5 wallet comparison list, sorted by jito_tokens descending
    let mut comparison_list: Vec<WalletComparison> = jito_accum.iter()
        .map(|(w, jt)| WalletComparison {
            wallet: w.clone(),
            jito_tokens: *jt,
            ooze_tokens: *ooze_accum.get(w).unwrap_or(&0.0),
        })
        .collect();
    comparison_list.sort_by(|a, b| b.jito_tokens.partial_cmp(&a.jito_tokens).unwrap_or(std::cmp::Ordering::Equal));
    comparison_list.truncate(5);

    // Price impact reduction estimate:
    // If coordination_pct of volume was driven by coordinated wallets, and Ooze
    // breaks their atomicity, the portion of the price move attributable to
    // coordination gets diluted by the ordering randomization.
    // Conservative estimate: the coordinated portion's price impact is halved under Ooze.
    let price_impact_reduction = event.coordination_pct * 0.5;

    OozeReplay {
        trades_in_window: event.trades.len(),
        jito_top_wallet: jito_wallet,
        jito_top_tokens: jito_tokens,
        jito_top_supply_pct: jito_tokens / TOTAL_SUPPLY * 100.0,
        ooze_top_wallet: ooze_wallet,
        ooze_top_tokens: ooze_tokens,
        ooze_top_supply_pct: ooze_tokens / TOTAL_SUPPLY * 100.0,
        reduction_pct: reduction,
        price_impact_reduction,
        wallet_comparisons: comparison_list,
        notes: vec![
            "Probabilistic ordering simulation: under Ooze, trades in same 2s window".into(),
            "randomize into sequential positions; later positions pay worse slippage (50% penalty avg).".into(),
            "Expected capture = (1/n × full) + ((n-1)/n × (1 - 0.5)). Does not model bundler retries or AMM curves exactly.".into(),
        ],
    }
}

// ══════════════════════════════════════════════════════════════════════
// MAIN ANALYSIS
// ══════════════════════════════════════════════════════════════════════

pub async fn analyze_token(api_key: &str, mint: &str) -> Result<ForensicsReport, Box<dyn std::error::Error>> {
    let client = Client::new();
    let mut api_calls = 0u32;

    println!("  [1] Token overview...");
    let overview = fetch_overview(&client, api_key, mint).await?;
    api_calls += 1;
    println!("      ✓ {} ({})", overview.token.name, overview.token.symbol);
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    println!("  [2] Top traders by profit...");
    let traders = fetch_top_traders(&client, api_key, mint).await?;
    api_calls += 1;
    println!("      ✓ {} profiteers", traders.len());
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // OHLCV — use 1m candles for tokens under 7 days, 5m for older
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let created_s = overview.token.creation.as_ref()
        .map(|c| c.created_time)
        .unwrap_or(now_s - 86400);
    let age_hours = (now_s.saturating_sub(created_s)) as f64 / 3600.0;

    let interval = if age_hours > 168.0 { "5m" } else { "1m" };
    println!("  [3] OHLCV history ({} candles)...", interval);
    let candles = fetch_chart(&client, api_key, mint, interval, created_s, now_s).await
        .unwrap_or_default();
    api_calls += 1;
    println!("      ✓ {} candles", candles.len());
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    let ath_candle = candles.iter()
        .max_by(|a, b| a.high.partial_cmp(&b.high).unwrap_or(std::cmp::Ordering::Equal))
        .cloned();
    let ath_mcap = ath_candle.as_ref().map(|c| c.high * TOTAL_SUPPLY).unwrap_or(0.0);

    // Detect events from candles
    println!("  [4] Detecting dramatic price events from candles...");
    let mut events = detect_events(&candles);
    let dramatic = events.iter().filter(|e| e.severity == "DRAMATIC").count();
    let major = events.iter().filter(|e| e.severity == "MAJOR").count();
    println!("      ✓ {} events ({} dramatic, {} major)", events.len(), dramatic, major);

    // Fetch trades for each event and analyze coordination
    println!("  [5] Analyzing trades at each event...");
    let event_count = events.len();
    for (idx, event) in events.iter_mut().enumerate() {
        println!(
            "      Event {}/{}: {} {:+.1}% at {:.1}h",
            idx + 1, event_count,
            event.event_type, event.price_change_pct,
            (event.start_time_ms / 1000).saturating_sub(created_s) as f64 / 3600.0,
        );
        let (trades, calls) = match fetch_trades_for_window(
            &client, api_key, mint,
            event.start_time_ms, event.end_time_ms,
        ).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("         × failed: {}", e);
                (Vec::new(), 0)
            }
        };
        api_calls += calls;
        event.trades = trades;
        event.trades_fetched = !event.trades.is_empty();
        analyze_event_coordination(event);
        println!(
            "         ✓ {} trades, {} wallets, {} clusters, {:.0}% coordinated",
            event.trades.len(), event.unique_wallets,
            event.clusters.len(), event.coordination_pct,
        );
    }

    // Run Ooze replay on top 3 events by magnitude
    println!("  [6] Running Ooze replay on top 3 events...");
    let mut by_magnitude: Vec<usize> = (0..events.len()).collect();
    by_magnitude.sort_by(|a, b| events[*b].abs_magnitude.partial_cmp(&events[*a].abs_magnitude).unwrap_or(std::cmp::Ordering::Equal));
    for &idx in by_magnitude.iter().take(3) {
        if events[idx].trades.is_empty() { continue; }
        events[idx].ooze_replay = Some(ooze_replay(&events[idx]));
    }
    println!("      ✓ Done");

    // Primary pool
    let primary_pool = overview.pools.iter()
        .max_by(|a, b| a.liquidity.usd.partial_cmp(&b.liquidity.usd).unwrap_or(std::cmp::Ordering::Equal))
        .cloned()
        .unwrap_or_else(|| overview.pools[0].clone());

    // Summary stats
    let events_with_coord = events.iter().filter(|e| e.coordination_pct > 30.0).count();
    let avg_coord = if !events.is_empty() {
        events.iter().map(|e| e.coordination_pct).sum::<f64>() / events.len() as f64
    } else { 0.0 };

    Ok(ForensicsReport {
        overview, top_traders: traders, primary_pool, candles,
        total_events_detected: events.len(),
        dramatic_events_count: dramatic,
        major_events_count: major,
        events_with_coordination: events_with_coord,
        avg_coordination_pct: avg_coord,
        events,
        age_hours,
        ath_candle, ath_mcap_usd: ath_mcap,
        api_calls_used: api_calls,
    })
}