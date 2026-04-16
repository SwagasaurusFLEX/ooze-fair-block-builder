# 🟢 Ooze — Fair Block Builder for Solana

> *Replacing Jito's auction-based transaction ordering with verifiable fair ordering*

## The Problem

98.1% of Solana blocks run through Jito's auction system. Solana's leader schedule is public, so bots know which validator produces the next block. 770 out of 773 validators run Jito — meaning when a bot submits a bundle, it lands nearly 100% of the time. This creates a pay-to-play block space market where:

- **Sandwich attacks** — a bot places its own buy before your trade and its own sell after, profiting from the price impact your trade creates
- **Frontrunning** — a bot sees your pending transaction and pays a higher priority fee to execute the same trade before you
- **Bundled launches** — a few actors use Jito bundles to buy a token from dozens of wallets simultaneously at launch, making it look like organic demand, then dumping the supply on retail buyers who thought the launch was fair
- **Coordinated sniping** — during the TrumpCoin launch, 62% of $280M in volume went through Jito bundles, meaning bots used guaranteed ordering to grab supply before retail could even land a transaction

The result: retail users systematically lose value on every trade, and token launches that appear organic are often controlled by a few wallets using bundles.

## The Solution

Ooze is an alternative block builder that uses **cryptographically randomized ordering** instead of fee-based auctions.

### How it works

1. **No bundles** — all transactions are individual. No one can guarantee execution order or coordinate across multiple wallets atomically.
2. **Tiered randomization** — transactions are grouped by priority fee tier, then **shuffled randomly within each tier** using ChaCha20 CSPRNG. Paying more gets you into a higher tier, but not a specific position.
3. **MEV fee redistribution (concept)** — the goal is to return a portion of priority fees collected on Ooze validator blocks back to all wallets that transacted in that block. This is in concept phase only — the mechanism for on-chain redistribution is being designed and has not been implemented yet.
4. **Verifiable entropy** — architecture designed to plug in VRF/QRNG sources for on-chain proof of fair ordering

### Why this stops MEV

**Sandwich attacks** require the attacker's buy to land before the victim and their sell to land after. With randomized ordering within tiers, the attacker cannot guarantee position. Our Monte Carlo simulations show **>83% reduction** in successful sandwich attacks.

**Bundled launches** are eliminated entirely — Ooze does not process bundles. Each transaction is treated individually. A bad actor trying to buy from 50 wallets simultaneously would have those 50 transactions scattered randomly across the block instead of executing atomically together.

**Frontrunning** becomes unreliable — paying a higher priority fee puts you in a higher tier, but your position within that tier is random. You can't guarantee you land before a specific transaction.

## Architecture

```
┌─────────────────────────────────────────────────┐
│              Agave Validator TPU                 │
│                                                  │
│  Fetch → SigVerify → [OOZE SCHEDULER] → PoH → Broadcast
│                            │                     │
│                    ┌───────┴────────┐            │
│                    │ Priority Tiers │            │
│                    │  ┌──────────┐  │            │
│                    │  │ Tier 1   │──┼─→ shuffle  │
│                    │  │ Tier 2   │──┼─→ shuffle  │
│                    │  │ Tier 3   │──┼─→ shuffle  │
│                    │  │ Tier 4   │──┼─→ shuffle  │
│                    │  └──────────┘  │            │
│                    │   ChaCha20     │            │
│                    │   CSPRNG       │            │
│                    └────────────────┘            │
└─────────────────────────────────────────────────┘
```

**Entropy roadmap:**
- **V1 (current):** ChaCha20Rng seeded from OS entropy — unpredictable to validator operators
- **V2:** VRF (Verifiable Random Function) for on-chain proof of fair ordering
- **V3:** Quantum entropy integration (hardware QRNG / quantum-inspired algorithms) for provably non-deterministic ordering

## Quick Start

```bash
# Clone and build
git clone https://github.com/SwagasaurusFLEX/ooze-fair-block-builder.git
cd ooze-fair-block-builder
cargo build --release

# Run the comparison demo
cargo run --release
```

This runs two scenarios (DEX sandwich attack, memecoin launch) through three ordering strategies and shows:
- Transaction execution order under each strategy
- MEV events detected
- Value extracted under Jito vs Ooze ordering
- Monte Carlo analysis of sandwich success rates across 1000 random orderings

## Project Structure

```
src/
├── types.rs          # Transaction, ordering result, MEV event types
├── jito_ordering.rs  # Jito/Agave-style priority auction (the status quo)
├── ooze_ordering.rs  # Ooze fair ordering with randomization
├── scenarios.rs      # Realistic transaction scenarios for comparison
├── main.rs           # Demo binary with colored output
└── lib.rs            # Module root
```

## Key Data (from our research)

| Metric | Value |
|--------|-------|
| Active Solana validators | 773 |
| Running Jito client | 770 (98.1%) |
| Validators lost since Oct 2024 | ~4,600 (85%) |
| Revenue gap top vs bottom tier | 920x |
| Jito bundle hit rate | ~100% (770/773 leaders run Jito) |
| Peak Jito tips (% of block revenue) | 50-66% (late 2024) |

Full analysis: [Solana Validator Economics Dashboard](https://solana-validator-dashboard-7jlzowd9tcm93cafoend66.streamlit.app/)

## Roadmap

- [x] Ordering comparison engine (this repo)
- [x] MEV detection and quantification
- [x] Monte Carlo sandwich probability analysis
- [ ] Real mempool data integration (Solana RPC websocket)
- [ ] MEV fee redistribution mechanism design (concept — not yet implemented)
- [ ] Agave validator client fork with Ooze scheduler
- [ ] Stake pool for fair-ordering validator delegation
- [ ] Quantum entropy source integration

## Built With

- **Rust** — same language as the Agave validator client
- **ChaCha20** — CSPRNG from the `rand_chacha` crate
- **Research** — backed by analysis of 773 validators, 15GB+ of block data

## License

MIT

## Author

Built for the [Colosseum Frontier Hackathon](https://colosseum.com/frontier) (April-May 2026)