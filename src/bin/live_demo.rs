use colored::*;
use ooze_fair_block_builder::{
    jito_ordering::order_jito_auction,
    live_data::fetch_live_transactions,
    ooze_ordering::{order_ooze_fair, OozeConfig},
    types::{OrderingResult, OrderingStrategy},
};
use std::env;

#[tokio::main]
async fn main() {
    // Get API key from environment variable or command line
    let api_key = env::var("HELIUS_API_KEY").unwrap_or_else(|_| {
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            args[1].clone()
        } else {
            eprintln!("{}", "Usage: cargo run --bin live_demo <HELIUS_API_KEY>".red());
            eprintln!("  Or set HELIUS_API_KEY environment variable");
            std::process::exit(1);
        }
    });

    println!("{}", "═".repeat(72).purple());
    println!(
        "{}",
        "  OOZE Live Block Analysis — Real Solana Data"
            .bold()
            .purple()
    );
    println!(
        "{}",
        "  Fetching a recent block and comparing ordering strategies"
            .dimmed()
    );
    println!("{}\n", "═".repeat(72).purple());

    // Fetch live transactions (DEX only, max 30 for clean output)
    println!("{}", "  ▸ Fetching live Solana block data...".cyan().bold());
    let (txs, slot) = match fetch_live_transactions(&api_key, true, 30).await {
        Ok(result) => result,
        Err(e) => {
            // If no DEX txs found, try without DEX filter
            println!("  No DEX transactions in latest block, trying all transactions...");
            match fetch_live_transactions(&api_key, false, 30).await {
                Ok(result) => result,
                Err(e2) => {
                    eprintln!("{}: {}", "Error fetching block".red().bold(), e2);
                    std::process::exit(1);
                }
            }
        }
    };

    if txs.is_empty() {
        println!("{}", "  No transactions found in recent blocks.".yellow());
        return;
    }

    println!(
        "\n  {} Analyzing {} transactions from slot {}\n",
        "▸".cyan().bold(),
        txs.len().to_string().bold(),
        slot.to_string().bold()
    );

   // Show the raw transactions
    println!("{}", "  Transactions captured:".bold());
    println!("  {}", "─".repeat(60).dimmed());
    for (i, tx) in txs.iter().enumerate() {
        let label = tx.label.as_deref().unwrap_or("unknown");
        let bundle_marker = if tx.is_bundle { " [BUNDLE]".red().to_string() } else { "".to_string() };
        println!(
            "  {:>2}. {:<45} fee: {:>8} | cu: {:>6}{}",
            i + 1,
            label,
            tx.priority_fee_lamports,
            tx.compute_units,
            bundle_marker
        );
    }

    // Run comparisons
    println!("\n{}", "─".repeat(72).dimmed());
    println!(
        "\n{}\n",
        "  ▸ Running ordering comparison...".cyan().bold()
    );

    let jito_result = order_jito_auction(&txs);
    let ooze_result = order_ooze_fair(&txs, &OozeConfig::default());
    let ooze_rebate_result = order_ooze_fair(
        &txs,
        &OozeConfig {
            enable_rebate: true,
            rebate_percentage: 0.5,
            ..Default::default()
        },
    );

    print_ordering(&jito_result);
    println!();
    print_ordering(&ooze_result);

    // Summary
    println!("\n{}", "  Summary — Live Block Analysis".bold().white());
    println!("  {}", "─".repeat(60).dimmed());
    println!(
        "  {:40} {:>12} {:>12}",
        "", "Jito", "Ooze"
    );
    println!(
        "  {:40} {:>12} {:>12}",
        "MEV events detected",
        jito_result.mev_events.len(),
        ooze_result.mev_events.len(),
    );
    println!(
        "  {:40} {:>12} {:>12}",
        "Value extracted (lamports)",
        jito_result.total_mev_extracted,
        ooze_result.total_mev_extracted,
    );
    println!(
        "  {:40} {:>12} {:>12}",
        "Total priority fees",
        jito_result.total_priority_fees,
        ooze_result.total_priority_fees,
    );
    println!(
        "  {:40} {:>12}",
        "Potential rebate (concept)",
        ooze_rebate_result.total_rebated,
    );

    // Show top fee payers
    println!("\n{}", "  Top 5 Priority Fee Payers:".bold().white());
    println!("  {}", "─".repeat(60).dimmed());
    let mut by_fee = txs.clone();
    by_fee.sort_by(|a, b| b.priority_fee_lamports.cmp(&a.priority_fee_lamports));
    for tx in by_fee.iter().take(5) {
        let label = tx.label.as_deref().unwrap_or("unknown");
        let signer_short = if tx.signer.len() > 8 {
            &tx.signer[..8]
        } else {
            &tx.signer
        };
        println!(
            "  {:>8} lamports — {} (signer: {}...)",
            tx.priority_fee_lamports, label, signer_short
        );
    }

    println!("\n{}", "═".repeat(72).purple());
    println!(
        "  {} Block {} analyzed. {} transactions compared.",
        "✓".green().bold(),
        slot,
        txs.len()
    );
    println!("{}", "═".repeat(72).purple());
}

fn print_ordering(result: &OrderingResult) {
    let header = match result.strategy {
        OrderingStrategy::JitoAuction => format!("  ┌─ {} ─┐", result.strategy).red().bold(),
        OrderingStrategy::OozeFairOrder => format!("  ┌─ {} ─┐", result.strategy).green().bold(),
        _ => format!("  ┌─ {} ─┐", result.strategy).cyan().bold(),
    };
    println!("{}", header);

    for (i, tx) in result.ordered_transactions.iter().enumerate() {
        let label = tx.label.as_deref().unwrap_or("unknown");
        let bundle_marker = if tx.is_bundle { " [BUNDLE]" } else { "" };
        println!(
            "  │ {:>2}. {:<40} fee: {:>8}{}",
            i + 1,
            label,
            tx.priority_fee_lamports,
            bundle_marker
        );
    }

    if !result.mev_events.is_empty() {
        println!("  │");
        println!(
            "  │ {} MEV events detected:",
            result.mev_events.len().to_string().red().bold()
        );
        for event in &result.mev_events {
            println!(
                "  │   ⚠ {} — {} lamports",
                event.description.red(),
                event.value_extracted_lamports
            );
        }
    } else {
        println!("  │");
        println!("  │ {} No MEV extraction detected", "✓".green().bold());
    }
    println!("  └{}┘", "─".repeat(55));
}
