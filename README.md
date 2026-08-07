# Ledger

A deterministic limit-order-book matching engine in Rust, running live in the
browser via WebAssembly, with a synthetic market trading against you.

**Live demo:** [dushyantbaroliya.github.io/ledger](https://dushyantbaroliya.github.io/ledger/)

## What this is

Ledger is a market microstructure lab. The core is an exchange-style matching
engine implementing strict **price-time priority**: better prices trade first,
ties break by arrival order, and trades always print at the resting order's
price. On top of it, a seeded flow simulator posts, crosses, and cancels orders
the way impatient and patient market participants do. Price *emerges from the
book*; nothing in the simulation sets it directly.

You can trade against the flow yourself: the order ticket goes through exactly
the same matching path as the simulated agents.

## Design invariants

These are the rules the engine is built around, in priority order:

1. **Integer arithmetic only.** Prices are ticks (`i64`), quantities are lots
   (`u64`). Floats never enter the matching path; floating-point prices are
   how backtests silently diverge across platforms.
2. **Deterministic replay.** The same command sequence produces a
   byte-identical event stream on every platform, including wasm32. This is
   enforced by a test, not a comment.
3. **Events are the source of truth.** Every book mutation is reported as a
   typed event (`Accepted | Rejected | Trade | Posted | Cancelled`). The UI,
   tape, and stats are pure consumers of that stream.
4. **The core has zero dependencies.** `lob-core` is auditable in one sitting:
   `std` collections, no crates, no macros.

## Architecture

```
crates/
  lob-core/    matching engine: no deps, no I/O, no unsafe
  lob-wasm/    wasm-bindgen bridge (serde events + flat Float64Array depth)
web/           Vite + React + TS terminal UI, Canvas rendering
```

- **Book structure:** per side, `BTreeMap<Price, VecDeque<RestingOrder>>` plus
  an id → (side, price) index for O(log n) cancels. A flat sorted vec would
  beat it on small books; that swap is a benchmark-driven decision for later,
  not a guess made now.
- **Order types:** limit and market; `GTC`, `IOC`, `FOK` (FOK is checked
  against reachable liquidity before any fill: all-or-nothing means the book
  is untouched on reject).
- **The WASM boundary:** events cross as plain JS objects (order-arrival rate,
  ergonomics win); depth snapshots cross as flat `Float64Array`s (60 fps
  render path, allocation-free win).
- **Rendering:** the depth ladder and price chart are hand-rolled Canvas.
  React renders chrome at ~8 Hz; the market renders at frame rate. A 60 fps
  market never causes 60 fps React reconciliation.

## Running it

```sh
# engine tests
cargo test

# build the wasm package (output: web/src/wasm/)
wasm-pack build crates/lob-wasm --target web --out-dir ../../web/src/wasm --no-typescript-check

# frontend
cd web && npm install && npm run dev
```

## Honest limitations (current state)

- The market is **synthetic**: seeded stochastic flow, not historical data.
  Replay of real tick data (LOBSTER/IEX format) is the next milestone.
- No strategy/backtest layer yet. The engine emits everything a backtester
  needs (deterministic events), but the PnL accounting on top doesn't exist
  yet.
- Single instrument, single-threaded by design. Concurrency belongs to a
  separate load-testing milestone, not to the matching core.

## Roadmap

- [ ] Historical tick-data replay (LOBSTER sample feeds)
- [ ] Strategy trait + backtest runner with published results (wins *and* losses)
- [ ] Latency benchmark harness with committed, reproducible numbers
- [ ] Multi-level market-by-order view
