//! Core scalar types for the order book.
//!
//! Prices are integer ticks and quantities are integer lots. Floating point
//! never enters the matching path: exchanges match on integers, and floats
//! would make replay non-deterministic across platforms.

/// Engine-assigned, strictly increasing identifier for every accepted order.
pub type OrderId = u64;

/// Price in integer ticks. The tick size (e.g. $0.01) is metadata owned by
/// the instrument definition, not the engine.
pub type Price = i64;

/// Quantity in integer lots.
pub type Qty = u64;

/// Strictly increasing sequence number assigned to every event the engine
/// emits. Two runs over the same input stream produce identical sequences.
pub type Seq = u64;

/// Logical timestamp supplied by the caller (nanoseconds in live use,
/// arbitrary ticks in simulation). The engine orders by arrival, not by this
/// value; it is carried through for the tape and analytics.
pub type Timestamp = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Bid,
    Ask,
}

impl Side {
    #[inline]
    pub fn opposite(self) -> Side {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

/// How long an order may remain working.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForce {
    /// Rest on the book until filled or cancelled.
    GoodTillCancel,
    /// Fill what crosses immediately; cancel the remainder.
    ImmediateOrCancel,
    /// Fill the entire quantity immediately or execute nothing at all.
    FillOrKill,
}

/// A resting order on the book. Only limit orders rest; market orders are
/// taker-only and never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestingOrder {
    pub id: OrderId,
    pub side: Side,
    pub price: Price,
    /// Quantity still open (original minus fills).
    pub remaining: Qty,
    /// Arrival timestamp supplied at submission.
    pub timestamp: Timestamp,
}

/// One aggregated price level as exposed to depth snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub price: Price,
    pub qty: Qty,
    pub order_count: u32,
}
