use crate::types::{MevEvent, MevType};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

// ── Solana Tracker API ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradesResponse {
    pub trades: Vec<Trade>,
    pub next_cursor: Option<u64>,
    pub has_next_page: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trade {
    pub tx: Option<String>,
    pub amount: Option<f64>,
    pub price_usd: Option<f64>,
    pub volume: Option<f64>,
    pub volume_sol: Option<f64>,
    #[serde(rename = "type")]
    pub trade_type: Option<String>,
    pub wallet: Option<String>,
    pub time: Option<u64>,
    pub program: Option<String>,
}

// ── Our types ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TokenSwap {
    pub signature: String,
    pub timestamp: u64,
    pub wallet: String,
    pub program: String,
    pub token_amount: f64,
    pub sol_amount: f64,
    pub price_usd: f64,
    pub direction: String,
    pub minutes_since_first: f64,
    /// Market cap at this trade = price_usd * 1B supply
    pub mcap_usd: f64,
}

#[derive(Debug, Clone)]
pub struct WalletProfile {
    pub address: String,
    pub buy_count: usize,
    pub sell_count: usize,
    pub total_sol_bought: f64,
    pub total_sol_sold: f64,
    pub total_tokens_bought: f64,
    pub total_tokens_sold: f64,
    pub net_profit_sol: f64,
    pub first_tx_minutes: f64,
    pub is_repeat: bool,
}

/// A detected dump event
#[derive(Debug, Clone)]
pub struct DumpEvent {
    pub peak_price_usd: f64,
    pub peak_mcap_usd: f64,
    pub peak_time_minutes: f64,
    pub dump_price_usd: f64,
    pub dump_mcap_usd: f64,
    pub dump_time_minutes: f64,
    pub drop_pct: f64,
    pub dump_window_start_idx: usize,
    pub dump_window_end_idx: usize,
}

pub struct ForensicsReport {
    pub token_mint: String,
    pub swaps: Vec<TokenSwap>,
    pub wallet_profiles: Vec<WalletProfile>,
    pub mev_events: Vec<MevEvent>,
    pub dump_events: Vec<DumpEvent>,
    pub total_swaps: usize,
    pub total_buys: usize,
    pub total_sells: usize,
    pub unique_wallets: usize,
    pub repeat_wallets: usize,
    pub total_sol_volume: f64,
    pub peak_price_usd: f64,
    pub peak_mcap_usd: f64,
    pub final_price_usd: f64,
    pub final_mcap_usd: f64,
    pub max_drop_pct: f64,
    pub time_window_minutes: f64,
    pub top5_concentration_pct: f64,
}

const TOTAL_SUPPLY: f64 = 1_000_000_000.0;

// ── API fetch ───────────────────────────────────────────────────────

async fn fetch_trades(
    client: &Client,
    api_key: &str,
    token_mint: &str,
    cursor: Option<u64>,
) -> Result<TradesResponse, Box<dyn std::error::Error>> {
    let mut url = format!("https://data.solanatracker.io/trades/{}", token_mint);
    if let Some(c) = cursor {
        url = format!("{}?cursor={}", url, c);
    }

    let raw = client.get(&url).header("x-api-key", api_key).send().await?;
    let status = raw.status();
    let body = raw.text().await?;

    if !status.is_success() {
        return Err(format!("API error ({}): {}", status, &body[..200.min(body.len())]).into());
    }

    let resp: TradesResponse = serde_json::from_str(&body)
        .map_err(|e| format!("JSON parse: {} — {}", e, &body[..200.min(body.len())]))?;
    Ok(resp)
}

// ── Convert trades ──────────────────────────────────────────────────

fn convert_trades(trades: &[Trade], first_time: u64) -> Vec<TokenSwap> {
    trades.iter().filter_map(|t| {
        let dir = t.trade_type.as_deref()?;
        if dir != "buy" && dir != "sell" { return None; }
        let ts = t.time.unwrap_or(0);
        let mins = if ts > first_time { (ts - first_time) as f64 / 60_000.0 } else { 0.0 };
        let price = t.price_usd.unwrap_or(0.0);

        Some(TokenSwap {
            signature: t.tx.clone().unwrap_or_default(),
            timestamp: ts,
            wallet: t.wallet.clone().unwrap_or_default(),
            program: t.program.clone().unwrap_or_default(),
            token_amount: t.amount.unwrap_or(0.0),
            sol_amount: t.volume_sol.unwrap_or(0.0),
            price_usd: price,
            direction: dir.to_string(),
            minutes_since_first: mins,
            mcap_usd: price * TOTAL_SUPPLY,
        })
    }).collect()
}

// ── Build profiles ──────────────────────────────────────────────────

fn build_profiles(swaps: &[TokenSwap]) -> Vec<WalletProfile> {
    let mut map: HashMap<String, WalletProfile> = HashMap::new();
    for s in swaps {
        let p = map.entry(s.wallet.clone()).or_insert(WalletProfile {
            address: s.wallet.clone(),
            buy_count: 0, sell_count: 0,
            total_sol_bought: 0.0, total_sol_sold: 0.0,
            total_tokens_bought: 0.0, total_tokens_sold: 0.0,
            net_profit_sol: 0.0, first_tx_minutes: s.minutes_since_first,
            is_repeat: false,
        });
        if s.direction == "buy" {
            p.buy_count += 1; p.total_sol_bought += s.sol_amount; p.total_tokens_bought += s.token_amount;
        } else {
            p.sell_count += 1; p.total_sol_sold += s.sol_amount; p.total_tokens_sold += s.token_amount;
        }
        if s.minutes_since_first < p.first_tx_minutes { p.first_tx_minutes = s.minutes_since_first; }
    }
    for p in map.values_mut() {
        p.is_repeat = (p.buy_count + p.sell_count) >= 2;
        p.net_profit_sol = p.total_sol_sold - p.total_sol_bought;
    }
    let mut v: Vec<WalletProfile> = map.into_values().collect();
    v.sort_by(|a, b| b.total_tokens_bought.partial_cmp(&a.total_tokens_bought).unwrap_or(std::cmp::Ordering::Equal));
    v
}

// ── Detect dumps (50%+ drop from peak) ──────────────────────────────

fn detect_dumps(swaps: &[TokenSwap]) -> Vec<DumpEvent> {
    let mut events = Vec::new();
    if swaps.is_empty() { return events; }

    let mut peak_price = 0.0f64;
    let mut peak_idx = 0usize;
    let mut in_dump = false;

    for (i, s) in swaps.iter().enumerate() {
        if s.price_usd <= 0.0 { continue; }

        if s.price_usd > peak_price {
            peak_price = s.price_usd;
            peak_idx = i;
            in_dump = false;
        }

        if peak_price > 0.0 && !in_dump {
            let drop = (1.0 - s.price_usd / peak_price) * 100.0;
            if drop >= 50.0 {
                // Found a 50%+ dump. Window: 15 min before dump to 15 min after
                let dump_time = s.minutes_since_first;
                let window_start = dump_time - 15.0;
                let window_end = dump_time + 15.0;

                let start_idx = swaps.iter().position(|x| x.minutes_since_first >= window_start).unwrap_or(0);
                let end_idx = swaps.iter().rposition(|x| x.minutes_since_first <= window_end).unwrap_or(swaps.len() - 1);

                events.push(DumpEvent {
                    peak_price_usd: peak_price,
                    peak_mcap_usd: peak_price * TOTAL_SUPPLY,
                    peak_time_minutes: swaps[peak_idx].minutes_since_first,
                    dump_price_usd: s.price_usd,
                    dump_mcap_usd: s.price_usd * TOTAL_SUPPLY,
                    dump_time_minutes: dump_time,
                    drop_pct: drop,
                    dump_window_start_idx: start_idx,
                    dump_window_end_idx: end_idx,
                });

                in_dump = true; // Don't re-trigger until new peak
            }
        }
    }
    events
}

// ── Detect MEV patterns ─────────────────────────────────────────────

fn detect_patterns(swaps: &[TokenSwap], profiles: &[WalletProfile]) -> Vec<MevEvent> {
    let mut events = Vec::new();

    // Coordinated buying in first 2 minutes
    let early: Vec<&TokenSwap> = swaps.iter().filter(|s| s.minutes_since_first <= 2.0 && s.direction == "buy").collect();
    if early.len() >= 5 {
        let unique: std::collections::HashSet<&str> = early.iter().map(|s| s.wallet.as_str()).collect();
        let sol: f64 = early.iter().map(|s| s.sol_amount).sum();
        let tok: f64 = early.iter().map(|s| s.token_amount).sum();
        events.push(MevEvent {
            event_type: MevType::CoordinatedBuy,
            extractor_tx: early[0].signature.clone(),
            victim_tx: "first 2 min".to_string(),
            value_extracted_lamports: (sol * 1e9) as u64,
            description: format!("{} wallets bought {:.0} tokens ({:.2} SOL) in first 2 min", unique.len(), tok, sol),
        });
    }

    // Pump & dump wallets
    for p in profiles {
        if p.total_tokens_bought > 0.0 && p.total_tokens_sold > p.total_tokens_bought * 0.5 && p.total_sol_bought > 0.01 {
            let sold_pct = p.total_tokens_sold / p.total_tokens_bought * 100.0;
            events.push(MevEvent {
                event_type: MevType::BundleExtraction,
                extractor_tx: format!("{}", &p.address[..8.min(p.address.len())]),
                victim_tx: "retail".to_string(),
                value_extracted_lamports: (p.net_profit_sol.max(0.0) * 1e9) as u64,
                description: format!("{}... spent {:.4} SOL, sold {:.0}% back for {:.4} SOL (net: {:+.4})",
                    &p.address[..8.min(p.address.len())], p.total_sol_bought, sold_pct, p.total_sol_sold, p.net_profit_sol),
            });
        }
    }

    // Heavy traders
    for p in profiles {
        if p.buy_count + p.sell_count >= 5 {
            events.push(MevEvent {
                event_type: MevType::RepeatSigner,
                extractor_tx: format!("{}", &p.address[..8.min(p.address.len())]),
                victim_tx: "".to_string(),
                value_extracted_lamports: 0,
                description: format!("{}... {} buys + {} sells ({:.4} SOL volume)",
                    &p.address[..8.min(p.address.len())], p.buy_count, p.sell_count,
                    p.total_sol_bought + p.total_sol_sold),
            });
        }
    }
    events
}

// ── Replay simulation ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub name: String,
    pub actors: Vec<ReplayActor>,
    pub final_price_sol: f64,
}

#[derive(Debug, Clone)]
pub struct ReplayActor {
    pub wallet: String,
    pub sol_in: f64,
    pub tokens_got: f64,
    pub sol_out: f64,
    pub net: f64,
    pub is_repeat: bool,
}

struct Curve { vt: f64, vs: f64, k: f64 }
impl Curve {
    fn new() -> Self { let vt = 1_073_000_191.0; let vs = 30.0; Self { vt, vs, k: vt * vs } }
    fn price(&self) -> f64 { self.vs / self.vt }
    fn buy(&mut self, sol: f64) -> f64 {
        if sol <= 0.0 { return 0.0; }
        let ns = self.vs + sol; let nt = self.k / ns;
        let out = self.vt - nt; self.vt = nt; self.vs = ns; out.max(0.0)
    }
    fn sell(&mut self, tok: f64) -> f64 {
        if tok <= 0.0 { return 0.0; }
        let nt = self.vt + tok; let ns = self.k / nt;
        let out = self.vs - ns; self.vt = nt; self.vs = ns; out.max(0.0)
    }
}

pub fn replay_jito(swaps: &[TokenSwap]) -> ReplayResult {
    let mut c = Curve::new();
    let mut groups: HashMap<String, Vec<&TokenSwap>> = HashMap::new();
    for s in swaps { groups.entry(s.wallet.clone()).or_default().push(s); }
    let mut sorted: Vec<(String, Vec<&TokenSwap>)> = groups.into_iter().collect();
    sorted.sort_by(|a, b| {
        let sa: f64 = a.1.iter().filter(|s| s.direction == "buy").map(|s| s.sol_amount).sum();
        let sb: f64 = b.1.iter().filter(|s| s.direction == "buy").map(|s| s.sol_amount).sum();
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });
    let order: Vec<&TokenSwap> = sorted.into_iter().flat_map(|(_, v)| v).collect();
    run_replay(&order, &mut c, "Jito (bundles grouped, biggest spender first)")
}

pub fn replay_ooze(swaps: &[TokenSwap], seed: u64) -> ReplayResult {
    use rand::seq::SliceRandom; use rand::SeedableRng; use rand_chacha::ChaCha20Rng;
    let mut c = Curve::new();
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut refs: Vec<&TokenSwap> = swaps.iter().collect();
    refs.shuffle(&mut rng);
    run_replay(&refs, &mut c, "Ooze (fully randomized, no bundles)")
}

fn run_replay(order: &[&TokenSwap], curve: &mut Curve, name: &str) -> ReplayResult {
    let mut actors: HashMap<String, ReplayActor> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for s in order { *counts.entry(s.wallet.clone()).or_insert(0) += 1; }
    for s in order {
        let a = actors.entry(s.wallet.clone()).or_insert(ReplayActor {
            wallet: s.wallet.clone(), sol_in: 0.0, tokens_got: 0.0, sol_out: 0.0, net: 0.0,
            is_repeat: counts.get(&s.wallet).copied().unwrap_or(0) >= 2,
        });
        if s.direction == "buy" && s.sol_amount > 0.0 {
            a.tokens_got += curve.buy(s.sol_amount); a.sol_in += s.sol_amount;
        } else if s.direction == "sell" && s.token_amount > 0.0 {
            a.sol_out += curve.sell(s.token_amount);
        }
    }
    let fp = curve.price();
    let mut v: Vec<ReplayActor> = actors.into_values().collect();
    for a in &mut v { a.net = a.sol_out - a.sol_in; }
    v.sort_by(|a, b| b.tokens_got.partial_cmp(&a.tokens_got).unwrap_or(std::cmp::Ordering::Equal));
    ReplayResult { name: name.to_string(), actors: v, final_price_sol: fp }
}

// ── Main entry ──────────────────────────────────────────────────────

pub async fn analyze_token_launch(
    api_key: &str,
    token_mint: &str,
    max_pages: usize,
) -> Result<ForensicsReport, Box<dyn std::error::Error>> {
    let client = Client::new();
    println!("  Fetching trades from Solana Tracker...");

    let mut all = Vec::new();
    let mut cursor: Option<u64> = None;
    for page in 1..=max_pages {
        println!("    Page {}...", page);
        let resp = fetch_trades(&client, api_key, token_mint, cursor).await?;
        let n = resp.trades.len();
        all.extend(resp.trades);
        if !resp.has_next_page.unwrap_or(false) || n == 0 { break; }
        cursor = resp.next_cursor;
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    }

    println!("  Fetched {} trades", all.len());
    if all.is_empty() { return Err("No trades found".into()); }

    all.reverse(); // chronological
    let first_time = all.first().and_then(|t| t.time).unwrap_or(0);
    let swaps = convert_trades(&all, first_time);
    println!("  Parsed {} swaps", swaps.len());
    if swaps.is_empty() { return Err("No swaps".into()); }

    let buys = swaps.iter().filter(|s| s.direction == "buy").count();
    let sells = swaps.iter().filter(|s| s.direction == "sell").count();
    let sol: f64 = swaps.iter().map(|s| s.sol_amount).sum();
    let peak = swaps.iter().map(|s| s.price_usd).fold(0.0f64, f64::max);
    let final_p = swaps.last().map(|s| s.price_usd).unwrap_or(0.0);
    let drop = if peak > 0.0 { (1.0 - final_p / peak) * 100.0 } else { 0.0 };

    let profiles = build_profiles(&swaps);
    let unique = profiles.len();
    let repeats = profiles.iter().filter(|p| p.is_repeat).count();
    let total_bought: f64 = profiles.iter().map(|p| p.total_tokens_bought).sum();
    let top5: f64 = profiles.iter().take(5).map(|p| p.total_tokens_bought).sum();
    let top5_pct = if total_bought > 0.0 { top5 / total_bought * 100.0 } else { 0.0 };

    let mev = detect_patterns(&swaps, &profiles);
    let dumps = detect_dumps(&swaps);
    let window = swaps.last().map(|s| s.minutes_since_first).unwrap_or(0.0);

    Ok(ForensicsReport {
        token_mint: token_mint.to_string(),
        swaps, wallet_profiles: profiles, mev_events: mev, dump_events: dumps,
        total_swaps: all.len(), total_buys: buys, total_sells: sells,
        unique_wallets: unique, repeat_wallets: repeats,
        total_sol_volume: sol,
        peak_price_usd: peak, peak_mcap_usd: peak * TOTAL_SUPPLY,
        final_price_usd: final_p, final_mcap_usd: final_p * TOTAL_SUPPLY,
        max_drop_pct: drop, time_window_minutes: window,
        top5_concentration_pct: top5_pct,
    })
}