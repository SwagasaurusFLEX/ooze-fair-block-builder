# 🟢 Ooze — Fair Block Builder for Solana

> *Replacing Jito's auction-based transaction ordering with verifiable fair ordering*

## The Problem

98.1% of Solana blocks run through Jito's auction system. This creates a pay-to-play block space market where MEV searchers can:

- **Sandwich attack** retail users by wrapping their trades in buy/sell bundles
- **Frontrun** any transaction by paying higher tips to the block engine
- **Extract value** from every block — during the TrumpCoin launch, 62% of $280M in volume used Jito bundles

The result: retail users systematically lose value on every trade.

## The Solution

Ooze is an alternative block builder that uses **cryptographically randomized ordering** instead of fee-based auctions.

### How it works

1. **No bundles** — all transactions compete equally, no atomic ordering guarantees for searchers
2. **Tiered randomization** — transactions are grouped by priority fee tier, then **shuffled randomly within each tier** using ChaCha20 CSPRNG
3. **MEV rebate** — any residual extraction value is redistributed to affected wallets
4. **Verifiable entropy** — architecture designed to plug in VRF/QRNG sources for on-chain proof of fair ordering

### Why this stops MEV

Sandwich attacks require **deterministic ordering** — the attacker MUST be positioned before AND after the victim. When ordering within a tier is random, this guarantee disappears. Our Monte Carlo simulations show **>95% reduction** in successful sandwich attacks.

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
- Value extracted vs rebated
- Monte Carlo analysis of sandwich success rates

## Project Structure

```
src/
├── types.rs          # Transaction, ordering result, MEV event types
├── jito_ordering.rs  # Jito/Agave-style priority auction (the status quo)
├── ooze_ordering.rs  # Ooze fair ordering with randomization + rebate
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
| Revenue gap top vs bottom | 920x |
| Peak Jito tips (% of block revenue) | 50-66% |

Full analysis: [Solana Validator Economics Dashboard](https://solana-validator-dashboard-7jlzowd9tcm93cafoend66.streamlit.app/)

## Roadmap

- [x] Ordering comparison engine (this repo)
- [x] MEV detection and quantification
- [x] Monte Carlo sandwich probability analysis
- [ ] Real mempool data integration (Solana RPC websocket)
- [ ] On-chain rebate program (Anchor/native Solana program)
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
