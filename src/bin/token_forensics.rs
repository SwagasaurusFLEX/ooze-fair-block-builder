use colored::*;
use ooze_fair_block_builder::token_forensics::{analyze_token, ForensicsReport};
use std::env;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let api_key = env::var("SOLTRACKER_API_KEY").unwrap_or_else(|_| {
        if args.len() > 2 { args[2].clone() }
        else { eprintln!("{}", "Set SOLTRACKER_API_KEY env var".red()); std::process::exit(1); }
    });
    let mint = if args.len() > 1 { args[1].clone() }
    else { eprintln!("{}", "Usage: token_forensics <MINT>".red()); std::process::exit(1); };

    println!("{}", "═".repeat(76).purple());
    println!("{}", "  OOZE — Token Forensics".bold().purple());
    println!("{}", "  Bundler-to-Dumper Cross-Reference Analysis".dimmed());
    println!("{}\n", "═".repeat(76).purple());

    let r = match analyze_token(&api_key, &mint).await {
        Ok(r) => r,
        Err(e) => { eprintln!("{}: {}", "Error".red().bold(), e); std::process::exit(1); }
    };

    print_vitals(&r);
    print_bundler_summary(&r);
    print_cross_reference(&r);
    print_bundler_details(&r);
    print_bottom_line(&r);
    print_ooze_framing(&r);
}

fn format_age(hours: f64) -> String {
    if hours < 1.0 { format!("{:.0}m", hours * 60.0) }
    else if hours < 24.0 { format!("{:.1}h", hours) }
    else { format!("{:.1}d", hours / 24.0) }
}

fn format_money(n: f64) -> String {
    if n.abs() >= 1_000_000.0 { format!("${:.2}M", n / 1_000_000.0) }
    else if n.abs() >= 1_000.0 { format!("${:.1}K", n / 1_000.0) }
    else { format!("${:.2}", n) }
}

fn format_number(n: f64) -> String {
    if n.abs() >= 1_000_000.0 { format!("{:.2}M", n / 1_000_000.0) }
    else if n.abs() >= 1_000.0 { format!("{:.1}K", n / 1_000.0) }
    else { format!("{:.0}", n) }
}

fn print_vitals(r: &ForensicsReport) {
    let t = &r.overview.token;
    let p = &r.primary_pool;
    let price_usd = p.price.usd.unwrap_or(0.0);
    let mcap_usd = p.market_cap.usd;

    println!("{}", format!("┌─── VITALS {}", "─".repeat(63)).cyan().bold());
    println!("  {} {}  ({})", t.name.bold().white(), format!("${}", t.symbol).cyan(), t.mint.dimmed());
    println!();
    println!("  {:<16} {}", "Price:".dimmed(), format!("${:.8}", price_usd).bold());
    println!("  {:<16} {}", "Market Cap:".dimmed(), format_money(mcap_usd).bold().green());
    println!("  {:<16} {}", "Liquidity:".dimmed(), format_money(p.liquidity.usd).bold());
    println!("  {:<16} {}", "Age:".dimmed(), format_age(r.age_hours).bold());
    println!("  {:<16} {}", "Holders:".dimmed(), format_number(r.overview.holders as f64).bold());
    println!("  {:<16} {}", "Primary venue:".dimmed(), p.market.clone().bold());

    // Txns
    println!();
    let total_txns = r.overview.txns;
    let buys = r.overview.buys;
    let sells = r.overview.sells;
    let sell_ratio = if total_txns > 0 { sells as f64 / total_txns as f64 * 100.0 } else { 0.0 };
    println!("  {:<16} {} ({} buys / {} sells — {:.1}% sells)",
        "Transactions:".dimmed(),
        format_number(total_txns as f64).bold(),
        format_number(buys as f64).green(),
        format_number(sells as f64).red(),
        sell_ratio,
    );
    if let Some(pool_txns) = &p.txns {
        println!("  {:<16} {} total  /  24h: {}",
            "Pool volume:".dimmed(),
            format_money(pool_txns.volume).bold(),
            format_money(pool_txns.volume_24h),
        );
    }

    // Price movement
    if let Some(e) = &r.overview.events {
        println!();
        print!("  {:<16} ", "Price changes:".dimmed());
        if let Some(c) = &e.h1 { print!("1h: {}  ", colorize_pct(c.pct)); }
        if let Some(c) = &e.h6 { print!("6h: {}  ", colorize_pct(c.pct)); }
        if let Some(c) = &e.h24 { print!("24h: {}", colorize_pct(c.pct)); }
        println!();
    }

    // Risk summary
    println!();
    if let Some(score) = r.overview.risk.score {
        let risk_label = format!("{}/10", score);
        let colored = if score >= 7 { risk_label.red().bold() }
            else if score >= 4 { risk_label.yellow().bold() }
            else { risk_label.green().bold() };
        println!("  {:<16} {}", "Risk score:".dimmed(), colored);
    }
    if let Some(t10) = r.overview.risk.top10 {
        let label = format!("{:.2}%", t10);
        let colored = if t10 > 50.0 { label.red().bold() } else { label.yellow() };
        println!("  {:<16} {}", "Top 10 hold:".dimmed(), colored);
    }
    if r.overview.risk.rugged {
        println!("  {:<16} {}", "Status:".dimmed(), "RUGGED".red().bold());
    }

    // Risk flags
    if !r.overview.risk.risks.is_empty() {
        println!();
        println!("  {}", "Risk flags:".dimmed());
        for risk in &r.overview.risk.risks {
            let icon = match risk.level.as_str() {
                "danger" => "⛔".to_string(),
                "warning" => "⚠ ".to_string(),
                _ => "• ".to_string(),
            };
            let line = format!("{} {}", risk.name, risk.description);
            let colored = if risk.level == "danger" { line.red() } else if risk.level == "warning" { line.yellow() } else { line.normal() };
            println!("    {} {}", icon, colored);
        }
    }
    println!("{}", format!("└{}", "─".repeat(73)).cyan());
}

fn colorize_pct(pct: f64) -> String {
    if pct > 0.0 { format!("+{:.1}%", pct).green().to_string() }
    else if pct < 0.0 { format!("{:.1}%", pct).red().to_string() }
    else { format!("{:.1}%", pct).normal().to_string() }
}

fn print_bundler_summary(r: &ForensicsReport) {
    println!("\n{}", format!("┌─── BUNDLER ACTIVITY {}", "─".repeat(53)).red().bold());
    println!("  {:<26} {}", "Bundlers detected:", r.bundler_count.to_string().red().bold());
    println!("  {:<26} {}", "Initial supply bundled:",
        format!("{:.2}%", r.bundler_total_initial_pct).red().bold());
    println!("  {:<26} {}", "Still held by bundlers:",
        format!("{:.2}%", r.bundler_total_current_pct).yellow());
    let dumped_pct = r.bundler_total_initial_pct - r.bundler_total_current_pct;
    if dumped_pct > 0.0 {
        println!("  {:<26} {}", "Dumped by bundlers:",
            format!("{:.2}% of supply", dumped_pct).red().bold());
    }
    println!("{}", format!("└{}", "─".repeat(73)).red());
}

fn print_cross_reference(r: &ForensicsReport) {
    println!("\n{}", format!("┌─── CROSS-REFERENCE: BUNDLERS vs TOP PROFITEERS {}", "─".repeat(26)).purple().bold());
    println!("  {}", "The question: of the most profitable wallets, how many were launch bundlers?".dimmed());
    println!();

    let top_n = 25.min(r.cross_ref.len());
    if top_n == 0 {
        println!("  No data");
        return;
    }

    println!("  {:>4} {:<10} {:>8} {:>10} {:>8} {:>8} {:>10}  {}",
        "#", "Wallet", "Init%", "Profit", "Buys", "Sells", "Sold%", "Flag");
    println!("  {}", "─".repeat(73).dimmed());

    let mut bundler_count_in_top = 0;
    let mut bundler_profit_in_top = 0.0;
    let mut non_bundler_profit_in_top = 0.0;

    for (i, row) in r.cross_ref.iter().take(top_n).enumerate() {
        if row.total_profit_usd <= 0.0 { continue; } // only profiteers

        let wallet = &row.wallet[..10.min(row.wallet.len())];
        let init = if row.is_bundler { format!("{:.2}%", row.initial_pct) } else { "—".to_string() };
        let profit = format_money(row.total_profit_usd);
        let profit_colored = if row.total_profit_usd > 10000.0 { profit.red().bold() }
            else if row.total_profit_usd > 1000.0 { profit.red() }
            else { profit.yellow() };

        let flag = if row.is_bundler {
            bundler_count_in_top += 1;
            bundler_profit_in_top += row.total_profit_usd;
            "🎯 BUNDLER".red().bold().to_string()
        } else {
            non_bundler_profit_in_top += row.total_profit_usd;
            "—".dimmed().to_string()
        };

        let wallet_colored = if row.is_bundler { wallet.red().to_string() } else { wallet.to_string() };

        println!("  {:>3}. {:<10} {:>8} {:>10} {:>8} {:>8} {:>9.0}%  {}",
            i + 1, wallet_colored, init, profit_colored, row.buys, row.sells, row.sold_pct_of_held, flag);
    }

    println!("  {}", "─".repeat(73).dimmed());
    let total_in_top = bundler_profit_in_top + non_bundler_profit_in_top;
    let bundler_share = if total_in_top > 0.0 { bundler_profit_in_top / total_in_top * 100.0 } else { 0.0 };

    println!("  Bundlers in top {}: {} ({:.0}% of top-tier profit = {} of {})",
        top_n,
        bundler_count_in_top.to_string().red().bold(),
        bundler_share,
        format_money(bundler_profit_in_top).red().bold(),
        format_money(total_in_top).bold()
    );
    println!("{}", format!("└{}", "─".repeat(73)).purple());
}

fn print_bundler_details(r: &ForensicsReport) {
    let bundlers = r.overview.risk.bundlers.as_ref();
    if bundlers.is_none() { return; }
    let b = bundlers.unwrap();
    if b.wallets.is_empty() { return; }

    println!("\n{}", format!("┌─── TOP BUNDLERS BY INITIAL SUPPLY {}", "─".repeat(39)).red().bold());
    println!("  {:<4} {:<10} {:>10} {:>10} {:>10}  {}",
        "#", "Wallet", "Init %", "Still %", "Profit", "Status");
    println!("  {}", "─".repeat(73).dimmed());

    // Sort bundlers by initial percentage
    let mut sorted: Vec<&_> = b.wallets.iter().collect();
    sorted.sort_by(|a, b| b.initial_percentage.partial_cmp(&a.initial_percentage).unwrap_or(std::cmp::Ordering::Equal));

    // Build lookup for cross-ref data
    let profit_lookup: std::collections::HashMap<&str, f64> = r.cross_ref.iter()
        .filter(|c| c.is_bundler)
        .map(|c| (c.wallet.as_str(), c.total_profit_usd))
        .collect();

    for (i, w) in sorted.iter().take(15).enumerate() {
        let wallet = &w.wallet[..10.min(w.wallet.len())];
        let init = format!("{:.3}%", w.initial_percentage);
        let still = format!("{:.3}%", w.percentage);
        let profit = profit_lookup.get(w.wallet.as_str()).copied().unwrap_or(0.0);
        let profit_str = if profit > 0.01 { format_money(profit).green().to_string() }
            else if profit < -0.01 { format_money(profit).red().to_string() }
            else { "—".dimmed().to_string() };

        // Status: did they dump?
        let status = if w.initial_percentage > 0.01 && w.percentage < w.initial_percentage * 0.1 {
            "DUMPED 90%+".red().bold().to_string()
        } else if w.initial_percentage > 0.01 && w.percentage < w.initial_percentage * 0.5 {
            "dumped 50%+".red().to_string()
        } else if w.initial_percentage > 0.01 {
            "still holding".yellow().to_string()
        } else {
            "—".dimmed().to_string()
        };

        println!("  {:>3}. {:<10} {:>10} {:>10} {:>10}  {}",
            i + 1, wallet, init, still, profit_str, status);
    }
    println!("{}", format!("└{}", "─".repeat(73)).red());
}

fn print_bottom_line(r: &ForensicsReport) {
    println!("\n{}", "═".repeat(76).bold().white());
    println!("{}", "  BOTTOM LINE".bold().white());
    println!("{}", "═".repeat(76).bold().white());

    // How much did bundlers extract vs lose
    let total_winners = r.total_bundler_profit_usd + r.total_non_bundler_profit_usd;
    let bundler_share_of_profit = if total_winners > 0.0 {
        r.total_bundler_profit_usd / total_winners * 100.0
    } else { 0.0 };

    println!();
    println!("  {} {} wallets bundled at launch, controlling {:.1}% of initial supply.",
        "•".purple().bold(),
        r.bundler_count.to_string().red().bold(),
        r.bundler_total_initial_pct,
    );

    if r.bundlers_in_top_profiteers > 0 {
        println!("  {} Of the top {} most profitable traders, {} are bundlers.",
            "•".purple().bold(),
            r.top_profiteers_checked,
            r.bundlers_in_top_profiteers.to_string().red().bold(),
        );
    }

    if r.total_bundler_profit_usd > 0.0 {
        println!("  {} Bundlers extracted {} ({:.1}% of all winning trades).",
            "•".purple().bold(),
            format_money(r.total_bundler_profit_usd).red().bold(),
            bundler_share_of_profit,
        );
    }

    if r.total_retail_loss_usd > 0.0 {
        println!("  {} Retail losers lost {} across non-bundler wallets.",
            "•".purple().bold(),
            format_money(r.total_retail_loss_usd).red().bold(),
        );
    }

    // The damning match
    if r.bundlers_in_top_profiteers > r.top_profiteers_checked / 4 {
        println!();
        println!("  {} {}",
            "→".red().bold(),
            format!("{:.0}% of top profiteers are bundlers. This was not retail speculation — it was coordinated extraction.",
                r.bundlers_in_top_profiteers as f64 / r.top_profiteers_checked.max(1) as f64 * 100.0).red().bold(),
        );
    }
}

fn print_ooze_framing(r: &ForensicsReport) {
    println!("\n{}", "═".repeat(76).purple());
    println!("{}", "  WHAT OOZE CHANGES".bold().purple());
    println!("{}", "═".repeat(76).purple());

    println!();
    println!("  Under Jito today, these {} bundler wallets coordinated atomic", r.bundler_count.to_string().red().bold());
    println!("  multi-wallet buys at launch. They acquired {:.1}% of initial supply",
        r.bundler_total_initial_pct);
    println!("  before any retail buyer saw the token.");

    println!();
    println!("  Under Ooze:");
    println!("    {} Transactions scatter randomly within priority tiers", "•".purple());
    println!("    {} Multi-wallet bundles cannot execute atomically", "•".purple());
    println!("    {} Retail buys land between coordinated buys", "•".purple());
    println!("    {} Initial supply distribution flattens", "•".purple());
    println!("    {} Price impact per attacker wallet increases", "•".purple());

    if r.total_bundler_profit_usd > 0.0 {
        println!();
        println!("  The {} extracted by bundlers on this single token",
            format_money(r.total_bundler_profit_usd).red().bold());
        println!("  is the extraction Ooze is designed to prevent.");
    }

    println!();
}