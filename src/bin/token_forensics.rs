use colored::*;
use ooze_fair_block_builder::token_forensics::{
    analyze_token_launch, replay_jito, replay_ooze, ForensicsReport,
};
use ooze_fair_block_builder::types::MevType;
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let api_key = env::var("SOLTRACKER_API_KEY").unwrap_or_else(|_| {
        if args.len() > 2 { args[2].clone() }
        else { eprintln!("{}", "Set SOLTRACKER_API_KEY env var".red()); std::process::exit(1); }
    });
    let token = if args.len() > 1 { args[1].clone() }
    else { eprintln!("{}", "Usage: token_forensics <MINT>".red()); std::process::exit(1); };

    println!("{}", "═".repeat(72).purple());
    println!("{}", "  OOZE Token Forensics — Launch & Dump Analysis".bold().purple());
    println!("{}", "  Powered by Solana Tracker API".dimmed());
    println!("{}\n", "═".repeat(72).purple());
    println!("  Token: {}\n", token.cyan());

    let r = match analyze_token_launch(&api_key, &token, 20).await {
        Ok(r) => r,
        Err(e) => { eprintln!("{}: {}", "Error".red().bold(), e); std::process::exit(1); }
    };

    print_overview(&r);
    print_launch_window(&r);
    print_dump_analysis(&r);
    print_top_wallets(&r);
    print_patterns(&r);
    print_replay_launch(&r);
}

fn print_overview(r: &ForensicsReport) {
    println!("\n{}", "═".repeat(72).purple());
    println!("{}", "  OVERVIEW".bold().white());
    println!("{}", "═".repeat(72).purple());
    println!("  Swaps:              {} ({} buys / {} sells)", r.total_swaps.to_string().bold(), r.total_buys.to_string().green(), r.total_sells.to_string().red());
    println!("  Unique wallets:     {}", r.unique_wallets.to_string().bold());
    println!("  Repeat wallets:     {}", if r.repeat_wallets > 0 { r.repeat_wallets.to_string().red().bold() } else { "0".green().bold() });
    println!("  SOL volume:         {:.2} SOL", r.total_sol_volume);
    println!("  Peak price:         ${:.6}  (mcap: ${:.0})", r.peak_price_usd, r.peak_mcap_usd);
    println!("  Final price:        ${:.6}  (mcap: ${:.0})", r.final_price_usd, r.final_mcap_usd);
    println!("  Max drop:           {}", if r.max_drop_pct > 50.0 { format!("{:.1}%", r.max_drop_pct).red().bold() } else { format!("{:.1}%", r.max_drop_pct).yellow().bold() });
    println!("  Window:             {:.1} min", r.time_window_minutes);
    println!("  Top 5 concentration:{}", if r.top5_concentration_pct > 50.0 { format!(" {:.1}%", r.top5_concentration_pct).red().bold() } else { format!(" {:.1}%", r.top5_concentration_pct).green().bold() });
    println!("  Dumps detected:     {}", if !r.dump_events.is_empty() { r.dump_events.len().to_string().red().bold() } else { "0".green().bold() });
}

fn print_launch_window(r: &ForensicsReport) {
    // First 30 minutes
    let launch: Vec<_> = r.swaps.iter().filter(|s| s.minutes_since_first <= 30.0).collect();
    if launch.is_empty() { return; }

    let buy_count = launch.iter().filter(|s| s.direction == "buy").count();
    let sell_count = launch.iter().filter(|s| s.direction == "sell").count();
    let buy_sol: f64 = launch.iter().filter(|s| s.direction == "buy").map(|s| s.sol_amount).sum();
    let sell_sol: f64 = launch.iter().filter(|s| s.direction == "sell").map(|s| s.sol_amount).sum();
    let start_mcap = launch.first().map(|s| s.mcap_usd).unwrap_or(0.0);
    let end_mcap = launch.last().map(|s| s.mcap_usd).unwrap_or(0.0);

    println!("\n{}", "═".repeat(72).green());
    println!("{}", "  LAUNCH WINDOW (first 30 minutes)".bold().green());
    println!("{}", "═".repeat(72).green());
    println!("  Trades: {} ({} buys / {} sells)", launch.len(), buy_count.to_string().green(), sell_count.to_string().red());
    println!("  Buy volume:  {:.4} SOL", buy_sol);
    println!("  Sell volume: {:.4} SOL", sell_sol);
    println!("  Mcap start:  ${:.0}", start_mcap);
    println!("  Mcap end:    ${:.0}", end_mcap);

    println!("\n  {:>6} {:>8} {:>5} {:>10} {:>10} {:>10}", "Min", "Wallet", "Dir", "SOL", "Tokens", "Mcap $");
    println!("  {}", "─".repeat(60).dimmed());
    for s in launch.iter().take(25) {
        let w = &s.wallet[..6.min(s.wallet.len())];
        let d = if s.direction == "buy" { "BUY".green() } else { "SELL".red() };
        println!("  {:>5.1}m {:>8} {:>5} {:>10.4} {:>10.0} {:>10.0}",
            s.minutes_since_first, w, d, s.sol_amount, s.token_amount, s.mcap_usd);
    }
    if launch.len() > 25 { println!("  ... and {} more", launch.len() - 25); }
}

fn print_dump_analysis(r: &ForensicsReport) {
    if r.dump_events.is_empty() {
        println!("\n  {} No 50%+ dumps detected in this data window.", "✓".green());
        return;
    }

    for (i, dump) in r.dump_events.iter().enumerate() {
        println!("\n{}", "═".repeat(72).red());
        println!("{}", format!("  DUMP #{} — {:.1}% CRASH DETECTED", i + 1, dump.drop_pct).bold().red());
        println!("{}", "═".repeat(72).red());
        println!("  Peak:  ${:.6} (mcap ${:.0}) at {:.1}m", dump.peak_price_usd, dump.peak_mcap_usd, dump.peak_time_minutes);
        println!("  Dump:  ${:.6} (mcap ${:.0}) at {:.1}m", dump.dump_price_usd, dump.dump_mcap_usd, dump.dump_time_minutes);
        println!("  Drop:  {}", format!("{:.1}%", dump.drop_pct).red().bold());
        println!("  Mcap wiped: ${:.0}", dump.peak_mcap_usd - dump.dump_mcap_usd);

        // Show trades in the dump window
        let window: Vec<_> = r.swaps[dump.dump_window_start_idx..=dump.dump_window_end_idx.min(r.swaps.len() - 1)].iter().collect();
        let w_buys = window.iter().filter(|s| s.direction == "buy").count();
        let w_sells = window.iter().filter(|s| s.direction == "sell").count();
        let w_buy_sol: f64 = window.iter().filter(|s| s.direction == "buy").map(|s| s.sol_amount).sum();
        let w_sell_sol: f64 = window.iter().filter(|s| s.direction == "sell").map(|s| s.sol_amount).sum();

        println!("\n  Dump window (±15 min): {} trades ({} buys / {} sells)", window.len(), w_buys, w_sells);
        println!("  Buy vol:  {:.4} SOL", w_buy_sol);
        println!("  Sell vol: {:.4} SOL  ({})", w_sell_sol,
            if w_sell_sol > w_buy_sol { format!("{:.1}x more sells", w_sell_sol / w_buy_sol.max(0.001)).red().to_string() }
            else { "balanced".to_string() });

        println!("\n  {:>6} {:>8} {:>5} {:>10} {:>10} {:>10}", "Min", "Wallet", "Dir", "SOL", "Tokens", "Mcap $");
        println!("  {}", "─".repeat(60).dimmed());
        for s in window.iter().take(20) {
            let w = &s.wallet[..6.min(s.wallet.len())];
            let d = if s.direction == "buy" { "BUY".green() } else { "SELL".red() };
            println!("  {:>5.1}m {:>8} {:>5} {:>10.4} {:>10.0} {:>10.0}",
                s.minutes_since_first, w, d, s.sol_amount, s.token_amount, s.mcap_usd);
        }
        if window.len() > 20 { println!("  ... and {} more in window", window.len() - 20); }

        // Replay the dump window specifically
        let window_swaps: Vec<_> = r.swaps[dump.dump_window_start_idx..=dump.dump_window_end_idx.min(r.swaps.len() - 1)].to_vec();
        if !window_swaps.is_empty() {
            let jito = replay_jito(&window_swaps);
            let ooze = replay_ooze(&window_swaps, 42);

            println!("\n  {} Replay of dump window:", "▶".purple().bold());
            print_replay_compact(&jito, &ooze);
        }
    }
}

fn print_replay_compact(jito: &ooze_fair_block_builder::token_forensics::ReplayResult, ooze: &ooze_fair_block_builder::token_forensics::ReplayResult) {
    let jp: f64 = jito.actors.iter().filter(|a| a.is_repeat && a.net > 0.0).map(|a| a.net).sum();
    let op: f64 = ooze.actors.iter().filter(|a| a.is_repeat && a.net > 0.0).map(|a| a.net).sum();

    // Top buyer comparison
    if let Some(jt) = jito.actors.first() {
        if let Some(ot) = ooze.actors.iter().find(|a| a.wallet == jt.wallet) {
            println!("    Top buyer ({}...):", &jt.wallet[..8.min(jt.wallet.len())]);
            println!("      Jito: {:.0} tokens for {:.4} SOL", jt.tokens_got, jt.sol_in);
            println!("      Ooze: {:.0} tokens for {:.4} SOL", ot.tokens_got, ot.sol_in);
            if jt.tokens_got > 0.0 {
                let diff = (jt.tokens_got - ot.tokens_got) / jt.tokens_got * 100.0;
                if diff.abs() > 1.0 {
                    println!("      → Ooze: {:.1}% {} tokens for top buyer", diff.abs(),
                        if diff > 0.0 { "fewer" } else { "more" });
                }
            }
        }
    }

    if jp > 0.001 || op > 0.001 {
        println!("    Repeat wallet profits:");
        println!("      Jito: {:.4} SOL", jp);
        println!("      Ooze: {:.4} SOL", op);
        if jp > op && jp > 0.0 {
            println!("      → Ooze reduces profit by {:.1}%", (1.0 - op / jp) * 100.0);
        }
    }
}

fn print_top_wallets(r: &ForensicsReport) {
    println!("\n{}", "─".repeat(72).dimmed());
    println!("{}", "  TOP BUYERS".bold().white());
    println!("{}", "─".repeat(72).dimmed());
    println!("  {:>8} {:>10} {:>8} {:>8} {:>10} {:>5}/{:<4}  {}", "Wallet", "Tokens", "SOL in", "SOL out", "Profit", "B", "S", "Flags");
    println!("  {}", "─".repeat(70).dimmed());

    for p in r.wallet_profiles.iter().take(12) {
        let w = &p.address[..8.min(p.address.len())];
        let mut flags = Vec::new();
        if p.is_repeat { flags.push("REPEAT".red().to_string()); }
        if p.first_tx_minutes <= 1.0 { flags.push("EARLY".cyan().to_string()); }
        if p.total_tokens_bought > 0.0 && p.total_tokens_sold > p.total_tokens_bought * 0.5 {
            flags.push("DUMPED".red().bold().to_string());
        }
        let f = if flags.is_empty() { "—".to_string() } else { flags.join(" ") };
        let profit = if p.net_profit_sol > 0.01 { format!("+{:.4}", p.net_profit_sol).green().to_string() }
            else if p.net_profit_sol < -0.01 { format!("{:.4}", p.net_profit_sol).red().to_string() }
            else { format!("{:.4}", p.net_profit_sol) };
        println!("  {:>8} {:>10.0} {:>8.4} {:>8.4} {:>10} {:>5}/{:<4}  {}",
            w, p.total_tokens_bought, p.total_sol_bought, p.total_sol_sold, profit, p.buy_count, p.sell_count, f);
    }
}

fn print_patterns(r: &ForensicsReport) {
    if r.mev_events.is_empty() { return; }

    // Count by type
    let coord = r.mev_events.iter().filter(|e| e.event_type == MevType::CoordinatedBuy).count();
    let dumps = r.mev_events.iter().filter(|e| e.event_type == MevType::BundleExtraction).count();
    let heavy = r.mev_events.iter().filter(|e| e.event_type == MevType::RepeatSigner).count();

    println!("\n{}", "─".repeat(72).dimmed());
    println!("{}", "  PATTERN SUMMARY".bold().red());
    println!("{}", "─".repeat(72).dimmed());
    if coord > 0 { println!("  {} Coordinated buys: {}", "⚠".red(), coord); }
    if dumps > 0 { println!("  {} Pump & dump wallets: {}", "⚠".red(), dumps); }
    if heavy > 0 { println!("  {} Heavy traders (5+ txs): {}", "⚠".red(), heavy); }

    // Show top 5 most profitable dumpers
    let mut dumpers: Vec<_> = r.mev_events.iter()
        .filter(|e| e.event_type == MevType::BundleExtraction && e.value_extracted_lamports > 0)
        .collect();
    dumpers.sort_by(|a, b| b.value_extracted_lamports.cmp(&a.value_extracted_lamports));

    if !dumpers.is_empty() {
        println!("\n  Top profitable dumpers:");
        for d in dumpers.iter().take(5) {
            println!("    {} {}", "→".red(), d.description);
        }
    }

    // Ooze explanation
    println!("\n{}", "═".repeat(72).purple());
    println!("{}", "  WHAT OOZE CHANGES".bold().white());
    println!("{}", "═".repeat(72).purple());
    if r.repeat_wallets > 0 {
        println!("  {} {} repeat wallets can't bundle — txs scatter randomly", "→".purple().bold(), r.repeat_wallets);
    }
    if coord > 0 {
        println!("  {} Coordinated buying breaks — no atomic multi-wallet execution", "→".purple().bold());
    }
    if dumps > 0 {
        println!("  {} {} dumpers get worse entry prices — retail gets better ones", "→".purple().bold(), dumps);
    }
}

fn print_replay_launch(r: &ForensicsReport) {
    // Replay just the launch window (first 30 min)
    let launch: Vec<_> = r.swaps.iter().filter(|s| s.minutes_since_first <= 30.0).cloned().collect();
    if launch.is_empty() { return; }

    let jito = replay_jito(&launch);
    let ooze = replay_ooze(&launch, 42);

    println!("\n{}", "═".repeat(72).purple());
    println!("{}", "  REPLAY — Launch window (first 30 min)".bold().white());
    println!("{}", "═".repeat(72).purple());

    for (result, is_jito) in [(&jito, true), (&ooze, false)] {
        let label = if is_jito { result.name.red().bold() } else { result.name.green().bold() };
        println!("\n  {}", label);
        println!("  {:>8} {:>10} {:>10} {:>10} {:>10}", "Wallet", "SOL in", "Tokens", "SOL out", "Profit");
        for a in result.actors.iter().take(6) {
            if a.sol_in == 0.0 && a.tokens_got == 0.0 && a.sol_out == 0.0 { continue; }
            let w = &a.wallet[..8.min(a.wallet.len())];
            let p = if a.net > 0.01 { format!("+{:.4}", a.net).green().to_string() }
                else if a.net < -0.01 { format!("{:.4}", a.net).red().to_string() }
                else { format!("{:.4}", a.net) };
            println!("  {:>8} {:>10.4} {:>10.0} {:>10.4} {:>10}", w, a.sol_in, a.tokens_got, a.sol_out, p);
        }
    }

    // Final comparison
    println!("\n{}", "─".repeat(72).dimmed());
    if let Some(jt) = jito.actors.first() {
        if let Some(ot) = ooze.actors.iter().find(|a| a.wallet == jt.wallet) {
            println!("  Top buyer ({}...):", &jt.wallet[..8.min(jt.wallet.len())]);
            println!("    Jito: {:.0} tokens | Ooze: {:.0} tokens", jt.tokens_got, ot.tokens_got);
            if jt.tokens_got > 0.0 {
                let diff = (jt.tokens_got - ot.tokens_got) / jt.tokens_got * 100.0;
                if diff.abs() > 1.0 {
                    println!("    → {:.1}% {} tokens under Ooze", diff.abs(), if diff > 0.0 { "fewer" } else { "more" });
                }
            }
        }
    }

    let jp: f64 = jito.actors.iter().filter(|a| a.is_repeat && a.net > 0.0).map(|a| a.net).sum();
    let op: f64 = ooze.actors.iter().filter(|a| a.is_repeat && a.net > 0.0).map(|a| a.net).sum();
    if jp > 0.001 {
        println!("  Repeat wallet profits: Jito {:.4} SOL → Ooze {:.4} SOL", jp, op);
        if jp > op { println!("    → Ooze reduces by {:.1}%", (1.0 - op / jp) * 100.0); }
    }
    println!();
}