use colored::*;
use ooze_fair_block_builder::token_forensics::{analyze_token, DramaticEvent, ForensicsReport};
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

    banner();
    println!("  Target: {}\n", mint.cyan());

    let r = match analyze_token(&api_key, &mint).await {
        Ok(r) => r,
        Err(e) => { eprintln!("\n{}: {}", "Error".red().bold(), e); std::process::exit(1); }
    };

    println!();
    section_vitals(&r);
    section_events_summary(&r);
    for (idx, ev) in r.events.iter().enumerate() {
        section_event_detail(idx + 1, ev, &r);
    }
    section_verdict(&r);
    section_ooze_pitch(&r);
}

fn banner() {
    println!("{}", "═".repeat(78).purple());
    println!("{}", "  OOZE — Event-Focused Token Forensics".bold().purple());
    println!("{}", "  Detecting coordination at dramatic price moments".dimmed());
    println!("{}", "═".repeat(78).purple());
}

fn fmt_money(n: f64) -> String {
    if n.abs() >= 1e6 { format!("${:.2}M", n/1e6) }
    else if n.abs() >= 1e3 { format!("${:.1}K", n/1e3) }
    else { format!("${:.2}", n) }
}
fn fmt_num(n: f64) -> String {
    if n.abs() >= 1e6 { format!("{:.2}M", n/1e6) }
    else if n.abs() >= 1e3 { format!("{:.1}K", n/1e3) }
    else { format!("{:.0}", n) }
}
fn fmt_age(h: f64) -> String {
    if h < 1.0 { format!("{:.0}m", h*60.0) }
    else if h < 24.0 { format!("{:.1}h", h) }
    else { format!("{:.1}d", h/24.0) }
}
fn fmt_event_time(event_ms: u64, created_s: u64) -> String {
    let event_s = event_ms / 1000;
    if event_s <= created_s { return "launch".to_string(); }
    let diff = event_s - created_s;
    if diff < 60 { format!("{}s", diff) }
    else if diff < 3600 { format!("{}m", diff / 60) }
    else { format!("{:.1}h", diff as f64 / 3600.0) }
}
fn colorize_pct(pct: f64) -> String {
    if pct > 0.0 { format!("+{:.1}%", pct).green().to_string() }
    else if pct < 0.0 { format!("{:.1}%", pct).red().to_string() }
    else { format!("{:.1}%", pct).normal().to_string() }
}

fn section_header(title: &str, color: &str) {
    let line = format!("┌─── {} ", title);
    let padding = 78usize.saturating_sub(line.chars().count());
    let full = format!("{}{}", line, "─".repeat(padding));
    let colored = match color {
        "cyan" => full.cyan().bold(),
        "red" => full.red().bold(),
        "purple" => full.purple().bold(),
        "yellow" => full.yellow().bold(),
        "green" => full.green().bold(),
        _ => full.white().bold(),
    };
    println!("{}", colored);
}

fn section_footer(color: &str) {
    let line = format!("└{}", "─".repeat(77));
    let colored = match color {
        "cyan" => line.cyan(),
        "red" => line.red(),
        "purple" => line.purple(),
        "yellow" => line.yellow(),
        "green" => line.green(),
        _ => line.normal(),
    };
    println!("{}", colored);
}

fn section_vitals(r: &ForensicsReport) {
    section_header("VITALS", "cyan");
    let t = &r.overview.token;
    let p = &r.primary_pool;
    let price = p.price.usd.unwrap_or(0.0);
    let mcap = p.market_cap.usd;

    println!("  {} {}  ({})", t.name.bold().white(), format!("${}", t.symbol).cyan(), t.mint.dimmed());
    println!();
    println!("  {:<18} {}", "Price:".dimmed(), format!("${:.8}", price).bold());
    println!("  {:<18} {}", "Market cap (now):".dimmed(), fmt_money(mcap).bold().green());
    if r.ath_mcap_usd > 0.0 {
        let mult = if mcap > 0.0 { r.ath_mcap_usd / mcap } else { 1.0 };
        println!("  {:<18} {} ({}x current)", "ATH market cap:".dimmed(),
            fmt_money(r.ath_mcap_usd).bold().red(),
            format!("{:.1}", mult).yellow());
    }
    println!("  {:<18} {}", "Liquidity:".dimmed(), fmt_money(p.liquidity.usd).bold());
    println!("  {:<18} {}", "Age:".dimmed(), fmt_age(r.age_hours).bold());
    println!("  {:<18} {}", "Holders:".dimmed(), fmt_num(r.overview.holders as f64).bold());
    println!("  {:<18} {}", "Primary venue:".dimmed(), p.market.bold());
    println!();
    println!("  {:<18} {} ({} buys / {} sells)",
        "Transactions:".dimmed(), fmt_num(r.overview.txns as f64).bold(),
        fmt_num(r.overview.buys as f64).green(), fmt_num(r.overview.sells as f64).red());
    if let Some(e) = &r.overview.events {
        print!("  {:<18} ", "Price change:".dimmed());
        if let Some(c) = &e.h1 { print!("1h {} · ", colorize_pct(c.pct)); }
        if let Some(c) = &e.h6 { print!("6h {} · ", colorize_pct(c.pct)); }
        if let Some(c) = &e.h24 { print!("24h {}", colorize_pct(c.pct)); }
        println!();
    }
    println!();
    if let Some(score) = r.overview.risk.score {
        let label = format!("{}/10", score);
        let colored = if score >= 7 { label.red().bold() } else if score >= 4 { label.yellow().bold() } else { label.green().bold() };
        println!("  {:<18} {}", "Risk score:".dimmed(), colored);
    }
    if let Some(t10) = r.overview.risk.top10 {
        let label = format!("{:.2}%", t10);
        let colored = if t10 > 50.0 { label.red().bold() } else { label.yellow() };
        println!("  {:<18} {}", "Top 10 hold:".dimmed(), colored);
    }
    if r.overview.risk.rugged {
        println!("  {:<18} {}", "Status:".dimmed(), "RUGGED".red().bold());
    }
    section_footer("cyan");
}

fn section_events_summary(r: &ForensicsReport) {
    println!();
    section_header("DRAMATIC PRICE EVENTS — SUMMARY", "purple");
    println!("  {}", "Events detected by scanning minute-resolution price candles.".dimmed());
    println!("  {}", "DRAMATIC ≥50% move · MAJOR ≥25% · MINOR = biggest moves available if token was calm.".dimmed());
    println!();

    if r.events.is_empty() {
        println!("  {} No significant price events detected.", "ⓘ".yellow());
        section_footer("purple");
        return;
    }

    let created_s = r.overview.token.creation.as_ref().map(|c| c.created_time).unwrap_or(0);

    for (i, e) in r.events.iter().enumerate() {
        let t = fmt_event_time(e.start_time_ms, created_s);
        let sev_c = match e.severity.as_str() {
            "DRAMATIC" => format!("[{}]", e.severity).red().bold(),
            "MAJOR" => format!("[{}]", e.severity).yellow().bold(),
            _ => format!("[{}]", e.severity).dimmed(),
        };
        let type_c = if e.event_type == "PUMP" { "PUMP".green().bold() } else { "DUMP".red().bold() };
        let coord_c = if !e.trades_fetched {
            "(no trades)".dimmed().to_string()
        } else if e.coordination_pct >= 30.0 {
            format!("{:.0}% coordinated", e.coordination_pct).red().bold().to_string()
        } else if e.coordination_pct >= 15.0 {
            format!("{:.0}% coordinated", e.coordination_pct).yellow().to_string()
        } else {
            format!("{:.0}% coordinated", e.coordination_pct).dimmed().to_string()
        };
        println!("    {:>2}. {} {} {} at {} — {} ({} trades, {})",
            i + 1,
            sev_c, type_c,
            colorize_pct(e.price_change_pct),
            t,
            coord_c,
            e.trades.len(),
            format!("{:.2} SOL", e.total_trade_sol).dimmed(),
        );
    }
    println!();
    println!("  Total: {} events, {} driven by ≥30% coordination (avg: {:.0}% across all events)",
        r.total_events_detected.to_string().bold(),
        r.events_with_coordination.to_string().red().bold(),
        r.avg_coordination_pct);
    section_footer("purple");
}

fn section_event_detail(idx: usize, e: &DramaticEvent, r: &ForensicsReport) {
    println!();
    let color = match e.severity.as_str() {
        "DRAMATIC" => "red",
        "MAJOR" => "yellow",
        _ => "cyan",
    };
    let header = format!("EVENT #{}: {} {} {:+.1}%",
        idx, e.severity, e.event_type, e.price_change_pct);
    section_header(&header, color);

    let created_s = r.overview.token.creation.as_ref().map(|c| c.created_time).unwrap_or(0);

    println!("  {:<22} {}", "When:".dimmed(),
        format!("{} after launch ({} candle{})",
            fmt_event_time(e.start_time_ms, created_s),
            e.candle_count,
            if e.candle_count == 1 { "" } else { "s" }
        ).bold());
    println!("  {:<22} ${:.8} → ${:.8}", "Price:".dimmed(), e.price_start, e.price_end);
    println!("  {:<22} ${:.8} (low) / ${:.8} (high)", "Range:".dimmed(), e.price_low, e.price_high);
    println!("  {:<22} {:.2} SOL", "Candle volume:".dimmed(), e.candle_volume_sol);

    if !e.trades_fetched {
        println!();
        println!("  {} No trades could be fetched for this window.", "ⓘ".yellow());
        section_footer(color);
        return;
    }

    println!();
    println!("  {:<22} {} trades", "Trade analysis:".dimmed(), e.trades.len());
    println!("  {:<22} {} unique wallets", "Participants:".dimmed(), e.unique_wallets);
    println!("  {:<22} {:.2} SOL total", "Window volume:".dimmed(), e.total_trade_sol);
    println!("  {:<22} {} clusters detected (3+ wallets same direction in 2 sec)",
        "Clusters:".dimmed(), e.clusters.len());

    if !e.clusters.is_empty() {
        for (ci, cl) in e.clusters.iter().enumerate() {
            let dir_c = if cl.direction == "buy" { cl.direction.green() } else { cl.direction.red() };
            println!("      {}) {} — {} wallets, {:.2} SOL",
                ci + 1, dir_c, cl.wallets.len(), cl.total_sol);
        }
    }

    println!();
    let coord_label = format!("{:.0}%", e.coordination_pct);
    let coord_c = if e.coordination_pct >= 30.0 { coord_label.red().bold() }
        else if e.coordination_pct >= 15.0 { coord_label.yellow().bold() }
        else { coord_label.dimmed() };
    println!("  {}: {} of this {} was driven by {} coordinated wallets ({:.2} of {:.2} SOL)",
        "HEADLINE".bold(),
        coord_c,
        format!("{} {:+.1}%", e.event_type, e.price_change_pct).bold(),
        e.coordinated_wallet_count.to_string().red().bold(),
        e.coordinated_sol, e.total_trade_sol);

    if e.coordination_pct >= 30.0 {
        println!();
        println!("  {} This price move was {} — not organic demand.",
            "⚠ ".red().bold(),
            "primarily caused by coordinated bundling".red().bold());
    } else if e.coordination_pct >= 15.0 {
        println!();
        println!("  {} Coordination present but not dominant. Mixed signals.", "ⓘ".yellow());
    }

    // Ooze replay if available
    if let Some(rep) = &e.ooze_replay {
        println!();
        println!("  {} {}", "▶".green().bold(), "Ooze ordering replay:".bold());
        println!("    Top wallet captured:");
        println!("      Jito actual:  {} accumulated {} tokens ({:.3}% of supply)",
            (&rep.jito_top_wallet[..8.min(rep.jito_top_wallet.len())]).red(),
            fmt_num(rep.jito_top_tokens), rep.jito_top_supply_pct);
        println!("      Ooze modeled: {} accumulated {} tokens ({:.3}% of supply)",
            (&rep.ooze_top_wallet[..8.min(rep.ooze_top_wallet.len())]).green(),
            fmt_num(rep.ooze_top_tokens), rep.ooze_top_supply_pct);
        if rep.reduction_pct > 1.0 {
            println!("      → {} acquires {:.1}% fewer tokens under Ooze",
                "Top wallet".bold(), rep.reduction_pct);
        }
        println!();
        println!("    Estimated price impact reduction: {:.0}%",
            rep.price_impact_reduction);
        println!("    {} Under Ooze, this {} move would likely be {:.0}% instead of {:.1}%.",
            "→".green(),
            e.event_type.to_lowercase(),
            e.abs_magnitude - rep.price_impact_reduction,
            e.abs_magnitude);
        println!();
        for note in &rep.notes {
            println!("    {} {}", "ⓘ".dimmed(), note.dimmed());
        }
    }

    section_footer(color);
}

fn section_verdict(r: &ForensicsReport) {
    println!();
    println!("{}", "═".repeat(78).bold().white());
    println!("{}", "  VERDICT".bold().white());
    println!("{}", "═".repeat(78).bold().white());
    println!();

    let heavily_coord = r.events_with_coordination;
    let total = r.total_events_detected;

    if total == 0 {
        println!("  {} Not enough data to judge.", "ⓘ".yellow());
        return;
    }

    if heavily_coord >= (total + 1) / 2 {
        println!("  {} {} of {} events were driven by ≥30% coordination.",
            "VERDICT:".red().bold(),
            heavily_coord.to_string().red().bold(), total);
        println!("  This token's price action was {}.",
            "primarily manufactured by bundling activity".red().bold().underline());
    } else if heavily_coord > 0 {
        println!("  {} {} of {} events showed heavy coordination.",
            "VERDICT:".yellow().bold(),
            heavily_coord.to_string().yellow().bold(), total);
        println!("  {} present alongside organic moves.",
            "Mixed — coordination".yellow().bold());
    } else {
        println!("  {} No events showed dominant coordination.",
            "VERDICT:".green().bold());
        println!("  Price action appears {}.",
            "mostly organic".green().bold());
    }

    println!();
    println!("  Average coordination share across events: {:.0}%", r.avg_coordination_pct);
    println!("  API calls used: {}", r.api_calls_used);
}

fn section_ooze_pitch(r: &ForensicsReport) {
    println!();
    println!("{}", "═".repeat(78).purple());
    println!("{}", "  WHAT OOZE CHANGES".bold().purple());
    println!("{}", "═".repeat(78).purple());
    println!();

    println!("  Under Jito today:");
    println!("    {} Coordinated bundles execute atomically, so a single wallet (or a group) can", "•".red());
    println!("    {} capture the best prices at dramatic moments.", "•".red());
    if r.events_with_coordination > 0 {
        println!("    {} {} of the {} dramatic events on this token were driven by ≥30% coordination.",
            "•".red(), r.events_with_coordination, r.total_events_detected);
    }

    println!();
    println!("  Under Ooze (tiered randomized ordering):");
    println!("    {} Multi-wallet bundles cannot execute as a unit at the same price", "•".green());
    println!("    {} Retail transactions interleave with coordinated orders", "•".green());
    println!("    {} Coordinated price impact gets diluted — fewer one-sided pumps/dumps", "•".green());
    println!("    {} Validators still earn priority fees — just fairly", "•".green());

    println!();
    println!("  {}", "Jito is not evil. Monopoly is. Ooze is an alternative.".italic());
    println!();
}