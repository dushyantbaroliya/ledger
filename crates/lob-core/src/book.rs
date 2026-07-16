//! The limit order book and matching engine.
//!
//! Matching is strict price-time priority: better prices trade first, and
//! within a price level, earlier arrivals trade first (FIFO). Trades always
//! print at the resting (maker) order's price.
//!
//! # Data structure
//!
//! Each side is a `BTreeMap<Price, VecDeque<RestingOrder>>` — a sorted map of
//! price levels, each holding a FIFO queue. An id → (side, price) index gives
//! cancels O(log n) access without scanning. A flat sorted-vec book would be
//! faster for small books, but the BTreeMap keeps inserts/removals O(log n)
//! under adversarial depth and reads clearly, which this crate values first;
//! the hot-path layout can be revisited with benchmarks, not guesses.

use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::events::{CancelReason, Event, RejectReason};
use crate::types::{Level, OrderId, Price, Qty, RestingOrder, Seq, Side, TimeInForce, Timestamp};

#[derive(Debug, Default)]
pub struct OrderBook {
    /// Bid levels; best bid is the *highest* key.
    bids: BTreeMap<Price, VecDeque<RestingOrder>>,
    /// Ask levels; best ask is the *lowest* key.
    asks: BTreeMap<Price, VecDeque<RestingOrder>>,
    /// Locates a resting order for cancellation.
    index: HashMap<OrderId, (Side, Price)>,
    next_order_id: OrderId,
    next_seq: Seq,
}

impl OrderBook {
    pub fn new() -> Self {
        Self::default()
    }

    // ---------------------------------------------------------------- public

    /// Submit a limit order. Returns the assigned id and every event the
    /// order produced, in match order.
    pub fn submit_limit(
        &mut self,
        side: Side,
        price: Price,
        qty: Qty,
        tif: TimeInForce,
        timestamp: Timestamp,
    ) -> (OrderId, Vec<Event>) {
        let id = self.alloc_id();
        let mut events = Vec::new();

        if qty == 0 {
            events.push(self.reject(id, RejectReason::ZeroQuantity, timestamp));
            return (id, events);
        }
        if tif == TimeInForce::FillOrKill && self.crossable_qty(side, Some(price)) < qty {
            events.push(self.reject(id, RejectReason::FillOrKillUnfillable, timestamp));
            return (id, events);
        }

        events.push(self.accept(id, side, Some(price), qty, timestamp));
        let remaining = self.match_incoming(id, side, Some(price), qty, timestamp, &mut events);

        if remaining > 0 {
            match tif {
                TimeInForce::GoodTillCancel => self.post(id, side, price, remaining, timestamp, &mut events),
                TimeInForce::ImmediateOrCancel => {
                    events.push(self.cancel_event(id, CancelReason::ImmediateOrCancelRemainder, remaining, timestamp))
                }
                // FOK with remaining > 0 is unreachable: fillability was
                // checked up front and nothing mutates the book in between.
                TimeInForce::FillOrKill => unreachable!("FOK verified fillable before matching"),
            }
        }
        (id, events)
    }

    /// Submit a market order: taker-only, never rests.
    pub fn submit_market(&mut self, side: Side, qty: Qty, timestamp: Timestamp) -> (OrderId, Vec<Event>) {
        let id = self.alloc_id();
        let mut events = Vec::new();

        if qty == 0 {
            events.push(self.reject(id, RejectReason::ZeroQuantity, timestamp));
            return (id, events);
        }
        if self.opposite(side).is_empty() {
            events.push(self.reject(id, RejectReason::NoLiquidity, timestamp));
            return (id, events);
        }

        events.push(self.accept(id, side, None, qty, timestamp));
        let remaining = self.match_incoming(id, side, None, qty, timestamp, &mut events);
        if remaining > 0 {
            events.push(self.cancel_event(id, CancelReason::MarketRemainder, remaining, timestamp));
        }
        (id, events)
    }

    /// Cancel a resting order. Returns `None` if the id is unknown or the
    /// order already left the book.
    pub fn cancel(&mut self, id: OrderId, timestamp: Timestamp) -> Option<Event> {
        let (side, price) = self.index.remove(&id)?;
        let book = self.side_mut(side);
        let level = book.get_mut(&price)?;
        let pos = level.iter().position(|o| o.id == id)?;
        let order = level.remove(pos).expect("position was just found");
        if level.is_empty() {
            book.remove(&price);
        }
        Some(self.cancel_event(id, CancelReason::UserRequested, order.remaining, timestamp))
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next_back().copied()
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    pub fn spread(&self) -> Option<Price> {
        Some(self.best_ask()? - self.best_bid()?)
    }

    /// Top `n` aggregated levels, best price first.
    pub fn depth(&self, side: Side, n: usize) -> Vec<Level> {
        let aggregate = |(price, orders): (&Price, &VecDeque<RestingOrder>)| Level {
            price: *price,
            qty: orders.iter().map(|o| o.remaining).sum(),
            order_count: orders.len() as u32,
        };
        match side {
            Side::Bid => self.bids.iter().rev().take(n).map(aggregate).collect(),
            Side::Ask => self.asks.iter().take(n).map(aggregate).collect(),
        }
    }

    /// Number of resting orders across both sides.
    pub fn open_orders(&self) -> usize {
        self.index.len()
    }

    // -------------------------------------------------------------- matching

    /// Match an incoming order against the opposite side. Returns the
    /// unmatched remainder. `limit` of `None` means a market order.
    fn match_incoming(
        &mut self,
        taker_id: OrderId,
        taker_side: Side,
        limit: Option<Price>,
        qty: Qty,
        timestamp: Timestamp,
        events: &mut Vec<Event>,
    ) -> Qty {
        let mut remaining = qty;

        while remaining > 0 {
            let Some(best) = self.best_opposite(taker_side) else { break };
            if !Self::crosses(taker_side, limit, best) {
                break;
            }

            // One fill per iteration, re-borrowing the level each time so the
            // borrow of the queue never overlaps the seq/index mutations below.
            let opposite = self.side_mut(taker_side.opposite());
            let level = opposite.get_mut(&best).expect("best price level exists");
            let maker = level.front_mut().expect("levels are removed when empty");

            let fill = remaining.min(maker.remaining);
            maker.remaining -= fill;
            remaining -= fill;
            let maker_id = maker.id;
            let maker_remaining = maker.remaining;

            if maker_remaining == 0 {
                level.pop_front();
                if level.is_empty() {
                    opposite.remove(&best);
                }
                self.index.remove(&maker_id);
            }

            let seq = self.alloc_seq();
            events.push(Event::Trade {
                seq,
                maker_id,
                taker_id,
                taker_side,
                price: best,
                qty: fill,
                maker_remaining,
                timestamp,
            });
        }
        remaining
    }

    /// Total opposite-side quantity reachable within `limit` (all liquidity
    /// for market orders). Used for the fill-or-kill pre-check.
    fn crossable_qty(&self, taker_side: Side, limit: Option<Price>) -> Qty {
        let levels: Box<dyn Iterator<Item = (&Price, &VecDeque<RestingOrder>)>> = match taker_side {
            Side::Bid => Box::new(self.asks.iter()),
            Side::Ask => Box::new(self.bids.iter().rev()),
        };
        levels
            .take_while(|(price, _)| Self::crosses(taker_side, limit, **price))
            .map(|(_, orders)| orders.iter().map(|o| o.remaining).sum::<Qty>())
            .sum()
    }

    #[inline]
    fn crosses(taker_side: Side, limit: Option<Price>, book_price: Price) -> bool {
        match (taker_side, limit) {
            (_, None) => true,
            (Side::Bid, Some(limit)) => book_price <= limit,
            (Side::Ask, Some(limit)) => book_price >= limit,
        }
    }

    // -------------------------------------------------------------- plumbing

    fn post(
        &mut self,
        id: OrderId,
        side: Side,
        price: Price,
        remaining: Qty,
        timestamp: Timestamp,
        events: &mut Vec<Event>,
    ) {
        self.side_mut(side).entry(price).or_default().push_back(RestingOrder {
            id,
            side,
            price,
            remaining,
            timestamp,
        });
        self.index.insert(id, (side, price));
        let seq = self.alloc_seq();
        events.push(Event::Posted { seq, id, side, price, qty: remaining, timestamp });
    }

    fn best_opposite(&self, taker_side: Side) -> Option<Price> {
        match taker_side {
            Side::Bid => self.best_ask(),
            Side::Ask => self.best_bid(),
        }
    }

    fn side_mut(&mut self, side: Side) -> &mut BTreeMap<Price, VecDeque<RestingOrder>> {
        match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        }
    }

    fn opposite(&self, taker_side: Side) -> &BTreeMap<Price, VecDeque<RestingOrder>> {
        match taker_side {
            Side::Bid => &self.asks,
            Side::Ask => &self.bids,
        }
    }

    fn alloc_id(&mut self) -> OrderId {
        self.next_order_id += 1;
        self.next_order_id
    }

    fn alloc_seq(&mut self) -> Seq {
        self.next_seq += 1;
        self.next_seq
    }

    fn accept(&mut self, id: OrderId, side: Side, price: Option<Price>, qty: Qty, timestamp: Timestamp) -> Event {
        let seq = self.alloc_seq();
        Event::Accepted { seq, id, side, price, qty, timestamp }
    }

    fn reject(&mut self, id: OrderId, reason: RejectReason, timestamp: Timestamp) -> Event {
        let seq = self.alloc_seq();
        Event::Rejected { seq, id, reason, timestamp }
    }

    fn cancel_event(&mut self, id: OrderId, reason: CancelReason, qty: Qty, timestamp: Timestamp) -> Event {
        let seq = self.alloc_seq();
        Event::Cancelled { seq, id, reason, qty, timestamp }
    }
}
