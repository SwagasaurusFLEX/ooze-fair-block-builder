use crate::types::SimTransaction;

/// Generate a realistic memecoin launch scenario.
///
/// This models what happens during a token launch on Raydium/Pump.fun:
/// - Retail users submitting buys at various priority fees
/// - MEV searchers submitting sandwich bundles via Jito
/// - A dev wallet doing the initial liquidity add
///
/// Based on real data: during the TrumpCoin launch, 62% of the $280M
/// volume used Jito bundles. This scenario models that pattern.
pub fn memecoin_launch_scenario() -> Vec<SimTransaction> {
    let pool_account = "TokenPool_OOZE_SOL".to_string();
    let amm_account = "RaydiumAMM_OOZE".to_string();
    let mut txs = Vec::new();
    let base_time: u64 = 1_000_000; // base timestamp in micros

    // --- Dev adds initial liquidity (arrives first) ---
    txs.push(SimTransaction {
        signature: "sig_dev_liq".into(),
        signer: "DevWallet_0xAA".into(),
        read_accounts: vec![],
        write_accounts: vec![pool_account.clone(), amm_account.clone()],
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
        compute_units: 200_000,
        arrival_time_us: base_time,
        is_bundle: false,
        bundle_id: None,
        bundle_position: None,
        label: Some("Dev: add liquidity".into()),
    });

    // --- Retail buyers (arrive over next 500ms) ---
    let retail_buyers = vec![
        ("RetailAlice", 10_000, 100_000, 50, "Alice: buy 0.5 SOL"),
        ("RetailBob", 25_000, 150_000, 120, "Bob: buy 1.2 SOL"),
        ("RetailCarol", 5_000, 100_000, 200, "Carol: buy 0.3 SOL"),
        ("RetailDave", 15_000, 120_000, 180, "Dave: buy 0.8 SOL"),
        ("RetailEve", 8_000, 100_000, 90, "Eve: buy 0.4 SOL"),
        ("RetailFrank", 30_000, 200_000, 250, "Frank: buy 2.0 SOL"),
        ("RetailGrace", 12_000, 110_000, 300, "Grace: buy 0.6 SOL"),
        ("RetailHank", 20_000, 150_000, 150, "Hank: buy 1.0 SOL"),
    ];

    for (signer, priority_fee, cu, offset_ms, label) in retail_buyers {
        txs.push(SimTransaction {
            signature: format!("sig_{}", signer.to_lowercase()),
            signer: signer.to_string(),
            read_accounts: vec![amm_account.clone()],
            write_accounts: vec![pool_account.clone()],
            base_fee_lamports: 5_000,
            priority_fee_lamports: priority_fee,
            compute_units: cu,
            arrival_time_us: base_time + (offset_ms * 1_000),
            is_bundle: false,
            bundle_id: None,
            bundle_position: None,
            label: Some(label.to_string()),
        });
    }

    // --- MEV Sandwich Bot (Jito bundle) ---
    // The bot sees Bob's 1.2 SOL buy in the mempool.
    // It creates a bundle: [bot_buy, bob_buy, bot_sell]
    // The bundle arrives AFTER Bob's tx but Jito places it BEFORE.
    //
    // Bot pays massive priority fee to win the Jito auction.
    // Bob's buy pushes the price up. Bot sells at higher price.

    // Sandwich front (buy before Bob)
    txs.push(SimTransaction {
        signature: "sig_sandwich_front".into(),
        signer: "MEV_Bot_Sandwich".into(),
        read_accounts: vec![amm_account.clone()],
        write_accounts: vec![pool_account.clone()],
        base_fee_lamports: 5_000,
        priority_fee_lamports: 500_000, // 500k lamports tip to win auction
        compute_units: 150_000,
        arrival_time_us: base_time + (200 * 1_000), // arrives AFTER Bob
        is_bundle: true,
        bundle_id: Some("bundle_sandwich_bob".into()),
        bundle_position: Some(0),
        label: Some("MEV Bot: sandwich front-buy".into()),
    });

    // Bob's tx gets included in the bundle (Jito block engine inserts it)
    // In reality, the bundle references Bob's tx hash
    // For simulation, we model it as the bot controlling Bob's position

    // Sandwich back (sell after Bob)
    txs.push(SimTransaction {
        signature: "sig_sandwich_back".into(),
        signer: "MEV_Bot_Sandwich".into(),
        read_accounts: vec![amm_account.clone()],
        write_accounts: vec![pool_account.clone()],
        base_fee_lamports: 5_000,
        priority_fee_lamports: 500_000,
        compute_units: 150_000,
        arrival_time_us: base_time + (200 * 1_000),
        is_bundle: true,
        bundle_id: Some("bundle_sandwich_bob".into()),
        bundle_position: Some(2), // position 2 = after Bob at position 1
        label: Some("MEV Bot: sandwich back-sell".into()),
    });

    // --- Sniper Bot (Jito bundle, frontrunning retail) ---
    // This bot just wants to buy before everyone else to get the lowest price.
    // Pays a huge tip to be first in the block.
    txs.push(SimTransaction {
        signature: "sig_sniper".into(),
        signer: "MEV_Bot_Sniper".into(),
        read_accounts: vec![amm_account.clone()],
        write_accounts: vec![pool_account.clone()],
        base_fee_lamports: 5_000,
        priority_fee_lamports: 1_000_000, // 1M lamports = biggest tip
        compute_units: 200_000,
        arrival_time_us: base_time + (300 * 1_000), // arrives LAST but pays most
        is_bundle: true,
        bundle_id: Some("bundle_snipe".into()),
        bundle_position: Some(0),
        label: Some("Sniper Bot: frontrun all retail".into()),
    });

    txs
}

/// Generate a simpler DEX swap scenario with a single sandwich attack.
/// Good for clear demo visualization.
pub fn simple_sandwich_scenario() -> Vec<SimTransaction> {
    let pool = "SOL_USDC_Pool".to_string();
    let amm = "Raydium_SOL_USDC".to_string();
    let base_time: u64 = 1_000_000;

    vec![
        // Victim: normal user swapping SOL -> USDC
        SimTransaction {
            signature: "sig_victim".into(),
            signer: "NormalUser".into(),
            read_accounts: vec![amm.clone()],
            write_accounts: vec![pool.clone()],
            base_fee_lamports: 5_000,
            priority_fee_lamports: 10_000,
            compute_units: 100_000,
            arrival_time_us: base_time,
            is_bundle: false,
            bundle_id: None,
            bundle_position: None,
            label: Some("User: swap 10 SOL -> USDC".into()),
        },
        // Attacker front: buy SOL before victim (drives price up)
        SimTransaction {
            signature: "sig_attack_front".into(),
            signer: "SandwichBot".into(),
            read_accounts: vec![amm.clone()],
            write_accounts: vec![pool.clone()],
            base_fee_lamports: 5_000,
            priority_fee_lamports: 200_000,
            compute_units: 120_000,
            arrival_time_us: base_time + 50_000, // arrives AFTER victim
            is_bundle: true,
            bundle_id: Some("sandwich_1".into()),
            bundle_position: Some(0),
            label: Some("Bot: front-buy (sandwich)".into()),
        },
        // Attacker back: sell SOL after victim (profits from price impact)
        SimTransaction {
            signature: "sig_attack_back".into(),
            signer: "SandwichBot".into(),
            read_accounts: vec![amm.clone()],
            write_accounts: vec![pool.clone()],
            base_fee_lamports: 5_000,
            priority_fee_lamports: 200_000,
            compute_units: 120_000,
            arrival_time_us: base_time + 50_000,
            is_bundle: true,
            bundle_id: Some("sandwich_1".into()),
            bundle_position: Some(2),
            label: Some("Bot: back-sell (sandwich)".into()),
        },
        // Other normal txs in the block
        SimTransaction {
            signature: "sig_normal_1".into(),
            signer: "Trader_A".into(),
            read_accounts: vec![amm.clone()],
            write_accounts: vec![pool.clone()],
            base_fee_lamports: 5_000,
            priority_fee_lamports: 15_000,
            compute_units: 100_000,
            arrival_time_us: base_time + 10_000,
            is_bundle: false,
            bundle_id: None,
            bundle_position: None,
            label: Some("Trader A: swap".into()),
        },
        SimTransaction {
            signature: "sig_normal_2".into(),
            signer: "Trader_B".into(),
            read_accounts: vec![],
            write_accounts: vec!["other_account".into()],
            base_fee_lamports: 5_000,
            priority_fee_lamports: 5_000,
            compute_units: 50_000,
            arrival_time_us: base_time + 20_000,
            is_bundle: false,
            bundle_id: None,
            bundle_position: None,
            label: Some("Trader B: unrelated tx".into()),
        },
    ]
}
