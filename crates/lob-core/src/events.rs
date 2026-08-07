//! The engine's event stream.
//!
//! Every mutation of the book is reported as an [`Event`]. The stream is the
//! single source of truth: the UI, the trade tape, analytics, and the
//! backtester are all pure consumers of it, and replaying the same input
//! commands yields a byte-identical stream.

use crate::types::{OrderId, Price, Qty, Seq, Side, Timestamp};

/// Why an order was rejected without touching the book.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Quantity of zero.
    ZeroQuantity,
    /// A fill-or-kill order whose full quantity could not be matched.
    FillOrKillUnfillable,
    /// A market order arriving when the opposite side of the book is empty.
    NoLiquidity,
}

/// Why an order left the book (or never rested).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelReason {
    /// Explicit cancel request from the order owner.
    UserRequested,
    /// Unfilled remainder of an immediate-or-cancel order.
    ImmediateOrCancelRemainder,
    /// Unfilled remainder of a market order that exhausted the book.
    MarketRemainder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The order passed validation and entered the matching path.
    Accepted {
        seq: Seq,
        id: OrderId,
        side: Side,
        price: Option<Price>,
        qty: Qty,
        timestamp: Timestamp,
    },
    /// The order failed validation; the book is untouched.
    Rejected {
        seq: Seq,
        id: OrderId,
        reason: RejectReason,
        timestamp: Timestamp,
    },
    /// A match. Price is always the resting (maker) order's price.
    Trade {
        seq: Seq,
        maker_id: OrderId,
        taker_id: OrderId,
        /// Side of the *taker*, the aggressor. A `Bid` taker means the trade
        /// printed at the ask (a lift); an `Ask` taker hit the bid.
        taker_side: Side,
        price: Price,
        qty: Qty,
        /// Maker's open quantity after this fill (0 = maker fully filled).
        maker_remaining: Qty,
        timestamp: Timestamp,
    },
    /// A limit order (or its unfilled remainder) is now resting on the book.
    Posted {
        seq: Seq,
        id: OrderId,
        side: Side,
        price: Price,
        qty: Qty,
        timestamp: Timestamp,
    },
    /// An order left the book unfilled (in whole or in part).
    Cancelled {
        seq: Seq,
        id: OrderId,
        reason: CancelReason,
        /// Quantity that was still open when cancelled.
        qty: Qty,
        timestamp: Timestamp,
    },
}

impl Event {
    pub fn seq(&self) -> Seq {
        match *self {
            Event::Accepted { seq, .. }
            | Event::Rejected { seq, .. }
            | Event::Trade { seq, .. }
            | Event::Posted { seq, .. }
            | Event::Cancelled { seq, .. } => seq,
        }
    }
}
