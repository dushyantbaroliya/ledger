//! Behavioural tests for the matching engine.
//!
//! Each test states the exchange rule it verifies. Prices are integer ticks;
//! read `10_000` as e.g. $100.00 with a $0.01 tick.

use lob_core::{CancelReason, Event, OrderBook, RejectReason, Side, TimeInForce};

const T: u64 = 0; // timestamp is carried through, not matched on

fn trades(events: &[Event]) -> Vec<(u64, u64, i64, u64)> {
    events
        .iter()
        .filter_map(|e| match *e {
            Event::Trade { maker_id, taker_id, price, qty, .. } => Some((maker_id, taker_id, price, qty)),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------- resting

#[test]
fn non_crossing_limit_rests_on_the_book() {
    let mut book = OrderBook::new();
    let (id, events) = book.submit_limit(Side::Bid, 10_000, 5, TimeInForce::GoodTillCancel, T);

    assert!(matches!(events[0], Event::Accepted { .. }));
    assert!(matches!(events[1], Event::Posted { price: 10_000, qty: 5, .. }));
    assert_eq!(book.best_bid(), Some(10_000));
    assert_eq!(book.best_ask(), None);
    assert_eq!(book.open_orders(), 1);
    assert!(id > 0);
}

#[test]
fn zero_quantity_is_rejected_without_touching_the_book() {
    let mut book = OrderBook::new();
    let (_, events) = book.submit_limit(Side::Bid, 10_000, 0, TimeInForce::GoodTillCancel, T);
    assert!(matches!(events[0], Event::Rejected { reason: RejectReason::ZeroQuantity, .. }));
    assert_eq!(book.open_orders(), 0);
}

// ---------------------------------------------------------------- priority

#[test]
fn better_price_has_priority_over_earlier_arrival() {
    let mut book = OrderBook::new();
    let (worse, _) = book.submit_limit(Side::Ask, 10_020, 5, TimeInForce::GoodTillCancel, T);
    let (better, _) = book.submit_limit(Side::Ask, 10_010, 5, TimeInForce::GoodTillCancel, T);

    let (taker, events) = book.submit_limit(Side::Bid, 10_020, 7, TimeInForce::GoodTillCancel, T);

    // The cheaper ask fills completely first, then the older-but-worse ask.
    assert_eq!(trades(&events), vec![(better, taker, 10_010, 5), (worse, taker, 10_020, 2)]);
}

#[test]
fn same_price_fills_in_fifo_order() {
    let mut book = OrderBook::new();
    let (first, _) = book.submit_limit(Side::Ask, 10_000, 5, TimeInForce::GoodTillCancel, T);
    let (second, _) = book.submit_limit(Side::Ask, 10_000, 5, TimeInForce::GoodTillCancel, T);

    let (taker, events) = book.submit_limit(Side::Bid, 10_000, 6, TimeInForce::GoodTillCancel, T);

    assert_eq!(trades(&events), vec![(first, taker, 10_000, 5), (second, taker, 10_000, 1)]);
}

#[test]
fn trade_prints_at_the_makers_price() {
    let mut book = OrderBook::new();
    let (maker, _) = book.submit_limit(Side::Ask, 10_000, 5, TimeInForce::GoodTillCancel, T);
    // Taker willing to pay up to 10_050 still executes at 10_000.
    let (taker, events) = book.submit_limit(Side::Bid, 10_050, 5, TimeInForce::GoodTillCancel, T);
    assert_eq!(trades(&events), vec![(maker, taker, 10_000, 5)]);
}

// ---------------------------------------------------------------- partials

#[test]
fn partial_fill_posts_the_remainder() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Ask, 10_000, 3, TimeInForce::GoodTillCancel, T);

    let (id, events) = book.submit_limit(Side::Bid, 10_000, 10, TimeInForce::GoodTillCancel, T);

    assert_eq!(trades(&events).iter().map(|t| t.3).sum::<u64>(), 3);
    assert!(matches!(*events.last().unwrap(), Event::Posted { id: pid, qty: 7, price: 10_000, .. } if pid == id));
    assert_eq!(book.best_bid(), Some(10_000));
    assert_eq!(book.best_ask(), None);
}

#[test]
fn partially_filled_maker_keeps_time_priority() {
    let mut book = OrderBook::new();
    let (maker_a, _) = book.submit_limit(Side::Ask, 10_000, 10, TimeInForce::GoodTillCancel, T);
    let (maker_b, _) = book.submit_limit(Side::Ask, 10_000, 10, TimeInForce::GoodTillCancel, T);

    book.submit_market(Side::Bid, 4, T); // A now has 6 left, still first in queue
    let (taker, events) = book.submit_market(Side::Bid, 8, T);

    assert_eq!(trades(&events), vec![(maker_a, taker, 10_000, 6), (maker_b, taker, 10_000, 2)]);
}

// ---------------------------------------------------------------- market

#[test]
fn market_order_sweeps_multiple_levels() {
    let mut book = OrderBook::new();
    let (a, _) = book.submit_limit(Side::Ask, 10_000, 2, TimeInForce::GoodTillCancel, T);
    let (b, _) = book.submit_limit(Side::Ask, 10_010, 2, TimeInForce::GoodTillCancel, T);
    let (c, _) = book.submit_limit(Side::Ask, 10_020, 2, TimeInForce::GoodTillCancel, T);

    let (taker, events) = book.submit_market(Side::Bid, 5, T);

    assert_eq!(trades(&events), vec![(a, taker, 10_000, 2), (b, taker, 10_010, 2), (c, taker, 10_020, 1)]);
    assert_eq!(book.best_ask(), Some(10_020));
}

#[test]
fn market_remainder_is_cancelled_never_posted() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Ask, 10_000, 3, TimeInForce::GoodTillCancel, T);

    let (_, events) = book.submit_market(Side::Bid, 10, T);

    assert!(matches!(
        *events.last().unwrap(),
        Event::Cancelled { reason: CancelReason::MarketRemainder, qty: 7, .. }
    ));
    assert_eq!(book.best_bid(), None, "market orders must never rest");
}

#[test]
fn market_order_into_empty_book_is_rejected() {
    let mut book = OrderBook::new();
    let (_, events) = book.submit_market(Side::Bid, 5, T);
    assert!(matches!(events[0], Event::Rejected { reason: RejectReason::NoLiquidity, .. }));
}

// ---------------------------------------------------------------- IOC / FOK

#[test]
fn ioc_fills_what_crosses_and_cancels_the_rest() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Ask, 10_000, 3, TimeInForce::GoodTillCancel, T);
    book.submit_limit(Side::Ask, 10_020, 5, TimeInForce::GoodTillCancel, T); // beyond limit

    let (_, events) = book.submit_limit(Side::Bid, 10_010, 10, TimeInForce::ImmediateOrCancel, T);

    assert_eq!(trades(&events).iter().map(|t| t.3).sum::<u64>(), 3);
    assert!(matches!(
        *events.last().unwrap(),
        Event::Cancelled { reason: CancelReason::ImmediateOrCancelRemainder, qty: 7, .. }
    ));
    assert_eq!(book.best_bid(), None, "IOC must never rest");
}

#[test]
fn fok_rejects_when_full_quantity_is_not_reachable() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Ask, 10_000, 3, TimeInForce::GoodTillCancel, T);
    book.submit_limit(Side::Ask, 10_020, 100, TimeInForce::GoodTillCancel, T); // outside limit

    let (_, events) = book.submit_limit(Side::Bid, 10_010, 5, TimeInForce::FillOrKill, T);

    assert_eq!(events.len(), 1, "all-or-nothing: no partial execution allowed");
    assert!(matches!(events[0], Event::Rejected { reason: RejectReason::FillOrKillUnfillable, .. }));
    assert_eq!(book.depth(Side::Ask, 10).iter().map(|l| l.qty).sum::<u64>(), 103, "book untouched");
}

#[test]
fn fok_executes_fully_when_liquidity_suffices() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Ask, 10_000, 3, TimeInForce::GoodTillCancel, T);
    book.submit_limit(Side::Ask, 10_010, 3, TimeInForce::GoodTillCancel, T);

    let (_, events) = book.submit_limit(Side::Bid, 10_010, 6, TimeInForce::FillOrKill, T);

    assert_eq!(trades(&events).iter().map(|t| t.3).sum::<u64>(), 6);
    assert_eq!(book.best_ask(), None);
}

// ---------------------------------------------------------------- cancel

#[test]
fn cancel_removes_the_order_and_reports_open_quantity() {
    let mut book = OrderBook::new();
    let (id, _) = book.submit_limit(Side::Bid, 10_000, 5, TimeInForce::GoodTillCancel, T);

    let event = book.cancel(id, T).expect("order is resting");
    assert!(matches!(event, Event::Cancelled { reason: CancelReason::UserRequested, qty: 5, .. }));
    assert_eq!(book.best_bid(), None);
    assert_eq!(book.open_orders(), 0);
}

#[test]
fn cancel_of_unknown_or_filled_order_is_a_noop() {
    let mut book = OrderBook::new();
    assert!(book.cancel(999, T).is_none());

    let (maker, _) = book.submit_limit(Side::Ask, 10_000, 5, TimeInForce::GoodTillCancel, T);
    book.submit_market(Side::Bid, 5, T); // fills maker completely
    assert!(book.cancel(maker, T).is_none(), "filled orders cannot be cancelled");
}

#[test]
fn cancelled_order_loses_queue_position() {
    let mut book = OrderBook::new();
    let (first, _) = book.submit_limit(Side::Ask, 10_000, 5, TimeInForce::GoodTillCancel, T);
    let (second, _) = book.submit_limit(Side::Ask, 10_000, 5, TimeInForce::GoodTillCancel, T);
    book.cancel(first, T);

    let (taker, events) = book.submit_market(Side::Bid, 5, T);
    assert_eq!(trades(&events), vec![(second, taker, 10_000, 5)]);
}

// ---------------------------------------------------------------- depth

#[test]
fn depth_aggregates_levels_best_price_first() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Bid, 9_990, 4, TimeInForce::GoodTillCancel, T);
    book.submit_limit(Side::Bid, 10_000, 3, TimeInForce::GoodTillCancel, T);
    book.submit_limit(Side::Bid, 10_000, 2, TimeInForce::GoodTillCancel, T);

    let depth = book.depth(Side::Bid, 10);
    assert_eq!(depth.len(), 2);
    assert_eq!((depth[0].price, depth[0].qty, depth[0].order_count), (10_000, 5, 2));
    assert_eq!((depth[1].price, depth[1].qty, depth[1].order_count), (9_990, 4, 1));
}

#[test]
fn spread_is_ask_minus_bid() {
    let mut book = OrderBook::new();
    book.submit_limit(Side::Bid, 9_990, 1, TimeInForce::GoodTillCancel, T);
    book.submit_limit(Side::Ask, 10_010, 1, TimeInForce::GoodTillCancel, T);
    assert_eq!(book.spread(), Some(20));
}

// ---------------------------------------------------------------- determinism

#[test]
fn identical_command_streams_produce_identical_event_streams() {
    let run = || {
        let mut book = OrderBook::new();
        let mut all = Vec::new();
        for i in 0..200u64 {
            // A fixed, branchy little scenario, no randomness needed to
            // prove replay: any divergence at all would fail equality.
            let side = if i % 3 == 0 { Side::Bid } else { Side::Ask };
            let price = 10_000 + ((i * 7) % 40) as i64 - 20;
            let (_, evs) = book.submit_limit(side, price, 1 + i % 5, TimeInForce::GoodTillCancel, i);
            all.extend(evs);
            if i % 11 == 0 {
                if let Some(e) = book.cancel(i / 2, i) {
                    all.push(e);
                }
            }
        }
        all
    };
    assert_eq!(run(), run());
}
