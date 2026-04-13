use colored::*;
use ooze_fair_block_builder::types::SimTransaction;
use ooze_fair_block_builder::{
    jito_ordering::order_jito_auction,
    ooze_ordering::{order_ooze_fair, OozeConfig},
    scenarios::{memecoin_launch_scenario, simple_sandwich_scenario},
    types::{OrderingResult, OrderingStrategy},
};

fn main() {
    println!("{}", "═".repeat(72).purple());
    println!(
        "{}",
        "  OOZE Fair Block Builder — Ordering Comparison Engine"
            .bold()
            .purple()
    );
    println!(
        "{}",
        "  Demonstrating why transaction ordering matters".dimmed()
    );
    println!("{}\n", "═".repeat(72).purple());

    // --- Scenario 1: Simple sandwich ---
    println!(
        "{}\n",
        "▸ Scenario 1: DEX Swap Sandwich Attack"
            .bold()
            .cyan()
    );
    let simple_txs = simple_sandwich_scenario();
    run_comparison(&simple_txs);

    println!("\n{}\n", "─".repeat(72).dimmed());

    // --- Scenario 2: Memecoin launch ---
    println!(
        "{}\n",
        "▸ Scenario 2: Memecoin Launch (modeled on real Jito data)"
            .bold()
            .cyan()
    );
    let launch_txs = memecoin_launch_scenario();
    run_comparison(&launch_txs);

    // --- Monte Carlo: run Ooze ordering 1000x to show sandwich probability ---
    println!("\n{}\n", "─".repeat(72).dimmed());
    println!(
        "{}\n",
        "▸ Monte Carlo: Sandwich success rate across 1000 random orderings"
            .bold()
            .cyan()
    );
    run_monte_carlo(&simple_sandwich_scenario(), 1000);
}

fn run_comparison(transactions: &[SimTransaction]) {
    
    // Run all three orderings
    let jito_result = order_jito_auction(transactions);
    let ooze_result = order_ooze_fair(
        transactions,
        &OozeConfig {
            demo_seed: Some(42), // fixed seed for reproducible demo
            ..Default::default()
        },
    );
    let ooze_rebate_result = order_ooze_fair(
        transactions,
        &OozeConfig {
            demo_seed: Some(42),
            enable_rebate: true,
            rebate_percentage: 0.5,
            ..Default::default()
        },
    );

    // Print each ordering
    print_ordering(&jito_result);
    println!();
    print_ordering(&ooze_result);
    println!();
    print_ordering(&ooze_rebate_result);

    // Summary comparison
    println!("\n{}", "  Summary Comparison".bold().white());
    println!("  {}", "─".repeat(60).dimmed());
    println!(
        "  {:40} {:>8} {:>8} {:>8}",
        "", "Jito", "Ooze", "Ooze+Rebate"
    );
    println!(
        "  {:40} {:>8} {:>8} {:>8}",
        "MEV events detected",
        jito_result.mev_events.len(),
        ooze_result.mev_events.len(),
        ooze_rebate_result.mev_events.len(),
    );
    println!(
        "  {:40} {:>8} {:>8} {:>8}",
        "Value extracted (lamports)",
        jito_result.total_mev_extracted,
        ooze_result.total_mev_extracted,
        ooze_rebate_result.total_mev_extracted,
    );
    println!(
        "  {:40} {:>8} {:>8} {:>8}",
        "Value rebated to users",
        jito_result.total_rebated,
        ooze_result.total_rebated,
        ooze_rebate_result.total_rebated,
    );
    println!(
        "  {:40} {:>8} {:>8} {:>8}",
        "Priority fees to validator",
        jito_result.total_priority_fees,
        ooze_result.total_priority_fees,
        ooze_rebate_result.total_priority_fees,
    );
}

fn print_ordering(result: &OrderingResult) {
    let color = match result.strategy {
        OrderingStrategy::JitoAuction => "red",
        OrderingStrategy::OozeFairOrder => "green",
        OrderingStrategy::OozeFairOrderWithRebate => "cyan",
        OrderingStrategy::Fcfs => "yellow",
    };

    let header = format!("  ┌─ {} ─┐", result.strategy);
    match color {
        "red" => println!("{}", header.red().bold()),
        "green" => println!("{}", header.green().bold()),
        "cyan" => println!("{}", header.cyan().bold()),
        _ => println!("{}", header.yellow().bold()),
    }

    for (i, tx) in result.ordered_transactions.iter().enumerate() {
        let label = tx.label.as_deref().unwrap_or(&tx.signature);
        let bundle_marker = if tx.is_bundle { " [BUNDLE]" } else { "" };
        println!(
            "  │ {:>2}. {:<45} fee: {:>8}{}", 
            i + 1, label, tx.priority_fee_lamports, bundle_marker
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
                "  │   ⚠ {} — extracted {} lamports",
                event.description.red(),
                event.value_extracted_lamports
            );
        }
    } else {
        println!("  │");
        println!(
            "  │ {} No MEV extraction detected",
            "✓".green().bold()
        );
    }
    println!("  └{}┘", "─".repeat(60));
}

fn run_monte_carlo(transactions: &[SimTransaction], iterations: usize) {
    
    let jito_result = order_jito_auction(transactions);
    let jito_sandwiches = jito_result.mev_events.len();

    let mut ooze_sandwich_count = 0;
    for i in 0..iterations {
        let result = order_ooze_fair(
            transactions,
            &OozeConfig {
                demo_seed: Some(i as u64),
                ..Default::default()
            },
        );
        ooze_sandwich_count += result.mev_events.len();
    }

    let ooze_rate = (ooze_sandwich_count as f64) / (iterations as f64);
    let jito_rate = jito_sandwiches as f64; // Jito is deterministic — always succeeds

    println!(
        "  Jito ordering:  sandwich succeeds {}/{} times ({}%)",
        jito_sandwiches.to_string().red().bold(),
        "1".red(),
        "100".red().bold()
    );
    println!(
        "  Ooze ordering:  sandwich succeeds {}/{} times ({:.1}%)",
        ooze_sandwich_count.to_string().green().bold(),
        iterations.to_string().green(),
        (ooze_rate * 100.0)
    );
    println!(
        "\n  {} MEV extraction reduced by {:.0}%",
        "→".purple().bold(),
        ((jito_rate - ooze_rate) / jito_rate * 100.0)
    );
}
