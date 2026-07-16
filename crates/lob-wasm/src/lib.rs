//! Browser bridge for the `lob-core` matching engine.
//!
//! Two serialization strategies, chosen by call frequency:
//! - **Events** (per submitted order) cross as plain JS objects via serde —
//!   ergonomic for the tape and stores, and cheap at order-arrival rate.
//! - **Depth snapshots** (per animation frame) cross as flat `Float64Array`s
//!   — `[price, qty, orderCount] × levels` — so the 60 fps render path never
//!   allocates JS objects or parses JSON.
//!
//! `lob-core` stays dependency-free; this crate mirrors its event types into
//! serde-able structs at the boundary instead of pushing derives inward.

use js_sys::Float64Array;
use lob_core::{CancelReason, Event, OrderBook, RejectReason, Side, TimeInForce};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Mirror of [`lob_core::Event`] shaped for JS consumption.
/// `type` discriminates; absent fields are simply not present on the object.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum JsEvent {
    #[serde(rename_all = "camelCase")]
    Accepted { seq: f64, id: f64, side: &'static str, price: Option<f64>, qty: f64, timestamp: f64 },
    #[serde(rename_all = "camelCase")]
    Rejected { seq: f64, id: f64, reason: &'static str, timestamp: f64 },
    #[serde(rename_all = "camelCase")]
    Trade {
        seq: f64,
        maker_id: f64,
        taker_id: f64,
        taker_side: &'static str,
        price: f64,
        qty: f64,
        maker_remaining: f64,
        timestamp: f64,
    },
    #[serde(rename_all = "camelCase")]
    Posted { seq: f64, id: f64, side: &'static str, price: f64, qty: f64, timestamp: f64 },
    #[serde(rename_all = "camelCase")]
    Cancelled { seq: f64, id: f64, reason: &'static str, qty: f64, timestamp: f64 },
}

fn side_str(side: Side) -> &'static str {
    match side {
        Side::Bid => "bid",
        Side::Ask => "ask",
    }
}

fn reject_str(reason: RejectReason) -> &'static str {
    match reason {
        RejectReason::ZeroQuantity => "zeroQuantity",
        RejectReason::FillOrKillUnfillable => "fokUnfillable",
        RejectReason::NoLiquidity => "noLiquidity",
    }
}

fn cancel_str(reason: CancelReason) -> &'static str {
    match reason {
        CancelReason::UserRequested => "userRequested",
        CancelReason::ImmediateOrCancelRemainder => "iocRemainder",
        CancelReason::MarketRemainder => "marketRemainder",
    }
}

fn mirror(event: &Event) -> JsEvent {
    match *event {
        Event::Accepted { seq, id, side, price, qty, timestamp } => JsEvent::Accepted {
            seq: seq as f64,
            id: id as f64,
            side: side_str(side),
            price: price.map(|p| p as f64),
            qty: qty as f64,
            timestamp: timestamp as f64,
        },
        Event::Rejected { seq, id, reason, timestamp } => JsEvent::Rejected {
            seq: seq as f64,
            id: id as f64,
            reason: reject_str(reason),
            timestamp: timestamp as f64,
        },
        Event::Trade { seq, maker_id, taker_id, taker_side, price, qty, maker_remaining, timestamp } => {
            JsEvent::Trade {
                seq: seq as f64,
                maker_id: maker_id as f64,
                taker_id: taker_id as f64,
                taker_side: side_str(taker_side),
                price: price as f64,
                qty: qty as f64,
                maker_remaining: maker_remaining as f64,
                timestamp: timestamp as f64,
            }
        }
        Event::Posted { seq, id, side, price, qty, timestamp } => JsEvent::Posted {
            seq: seq as f64,
            id: id as f64,
            side: side_str(side),
            price: price as f64,
            qty: qty as f64,
            timestamp: timestamp as f64,
        },
        Event::Cancelled { seq, id, reason, qty, timestamp } => JsEvent::Cancelled {
            seq: seq as f64,
            id: id as f64,
            reason: cancel_str(reason),
            qty: qty as f64,
            timestamp: timestamp as f64,
        },
    }
}

fn parse_side(side: &str) -> Result<Side, JsError> {
    match side {
        "bid" => Ok(Side::Bid),
        "ask" => Ok(Side::Ask),
        other => Err(JsError::new(&format!("unknown side: {other:?} (expected \"bid\" | \"ask\")"))),
    }
}

fn parse_tif(tif: &str) -> Result<TimeInForce, JsError> {
    match tif {
        "gtc" => Ok(TimeInForce::GoodTillCancel),
        "ioc" => Ok(TimeInForce::ImmediateOrCancel),
        "fok" => Ok(TimeInForce::FillOrKill),
        other => Err(JsError::new(&format!("unknown tif: {other:?} (expected \"gtc\" | \"ioc\" | \"fok\")"))),
    }
}

fn events_to_js(events: &[Event]) -> Result<JsValue, JsError> {
    let mirrored: Vec<JsEvent> = events.iter().map(mirror).collect();
    serde_wasm_bindgen::to_value(&mirrored).map_err(|e| JsError::new(&e.to_string()))
}

/// A single-instrument exchange: one order book plus its event history hooks.
#[wasm_bindgen]
pub struct Exchange {
    book: OrderBook,
}

/// Result of a submission: the assigned order id plus the events it produced.
#[wasm_bindgen]
pub struct Submission {
    id: f64,
    events: JsValue,
}

#[wasm_bindgen]
impl Submission {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> f64 {
        self.id
    }

    /// Array of event objects, in match order.
    #[wasm_bindgen(getter)]
    pub fn events(&self) -> JsValue {
        self.events.clone()
    }
}

#[wasm_bindgen]
impl Exchange {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Exchange {
        Exchange { book: OrderBook::new() }
    }

    /// Submit a limit order. `side`: "bid" | "ask"; `tif`: "gtc" | "ioc" | "fok".
    /// Prices are integer ticks, quantities integer lots (validated).
    #[wasm_bindgen(js_name = submitLimit)]
    pub fn submit_limit(
        &mut self,
        side: &str,
        price: f64,
        qty: f64,
        tif: &str,
        timestamp: f64,
    ) -> Result<Submission, JsError> {
        let (side, tif) = (parse_side(side)?, parse_tif(tif)?);
        let price = as_ticks(price)?;
        let qty = as_lots(qty)?;
        let (id, events) = self.book.submit_limit(side, price, qty, tif, timestamp as u64);
        Ok(Submission { id: id as f64, events: events_to_js(&events)? })
    }

    /// Submit a market order (taker-only; never rests).
    #[wasm_bindgen(js_name = submitMarket)]
    pub fn submit_market(&mut self, side: &str, qty: f64, timestamp: f64) -> Result<Submission, JsError> {
        let side = parse_side(side)?;
        let qty = as_lots(qty)?;
        let (id, events) = self.book.submit_market(side, qty, timestamp as u64);
        Ok(Submission { id: id as f64, events: events_to_js(&events)? })
    }

    /// Cancel a resting order. Returns the Cancelled event, or `null` if the
    /// order is unknown or already off the book.
    pub fn cancel(&mut self, id: f64, timestamp: f64) -> Result<JsValue, JsError> {
        match self.book.cancel(id as u64, timestamp as u64) {
            Some(event) => serde_wasm_bindgen::to_value(&mirror(&event)).map_err(|e| JsError::new(&e.to_string())),
            None => Ok(JsValue::NULL),
        }
    }

    /// Flat depth snapshot: `[price, qty, orderCount] × levels`, best first.
    pub fn depth(&self, side: &str, levels: usize) -> Result<Float64Array, JsError> {
        let side = parse_side(side)?;
        let depth = self.book.depth(side, levels);
        let mut flat = Vec::with_capacity(depth.len() * 3);
        for level in depth {
            flat.push(level.price as f64);
            flat.push(level.qty as f64);
            flat.push(level.order_count as f64);
        }
        Ok(Float64Array::from(flat.as_slice()))
    }

    #[wasm_bindgen(js_name = bestBid)]
    pub fn best_bid(&self) -> Option<f64> {
        self.book.best_bid().map(|p| p as f64)
    }

    #[wasm_bindgen(js_name = bestAsk)]
    pub fn best_ask(&self) -> Option<f64> {
        self.book.best_ask().map(|p| p as f64)
    }

    pub fn spread(&self) -> Option<f64> {
        self.book.spread().map(|s| s as f64)
    }

    #[wasm_bindgen(js_name = openOrders)]
    pub fn open_orders(&self) -> usize {
        self.book.open_orders()
    }
}

impl Default for Exchange {
    fn default() -> Self {
        Self::new()
    }
}

fn as_ticks(price: f64) -> Result<i64, JsError> {
    if !price.is_finite() || price.fract() != 0.0 {
        return Err(JsError::new("price must be an integer number of ticks"));
    }
    Ok(price as i64)
}

fn as_lots(qty: f64) -> Result<u64, JsError> {
    if !qty.is_finite() || qty.fract() != 0.0 || qty < 0.0 {
        return Err(JsError::new("qty must be a non-negative integer number of lots"));
    }
    Ok(qty as u64)
}
