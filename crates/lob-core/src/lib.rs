//! # lob-core
//!
//! A deterministic price-time-priority limit order book and matching engine.
//!
//! Design invariants:
//! - **Integer arithmetic only.** Prices are ticks, quantities are lots.
//! - **Deterministic replay.** The same command sequence always produces the
//!   same event stream, byte for byte, on every platform (including wasm32).
//! - **Events are the source of truth.** The book never reports state changes
//!   any way other than the [`events::Event`] stream.
//! - **Zero dependencies.** The engine is auditable in one sitting.

mod book;
pub mod events;
pub mod types;

pub use book::OrderBook;
pub use events::{CancelReason, Event, RejectReason};
pub use types::{Level, OrderId, Price, Qty, RestingOrder, Seq, Side, TimeInForce, Timestamp};
