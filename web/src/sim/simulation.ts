/**
 * The live simulation session: engine + order flow + derived state.
 *
 * Rendering is decoupled from React: canvases subscribe and repaint every
 * animation frame straight from this object; React chrome (top bar, stats)
 * subscribes too but re-renders at a throttled rate. State lives here, not
 * in component state, so a 60 fps market never causes 60 fps reconciliation.
 */
import type { Engine, EngineEvent, Side, Tif } from "./engine";
import { DEFAULT_MARKET, MarketFlow } from "./market";

export interface Trade {
  seq: number;
  price: number;
  qty: number;
  takerSide: Side;
  timestamp: number;
}

export interface HistoryPoint {
  mid: number | null;
  trade: { price: number; side: Side } | null;
}

export type Speed = 1 | 4 | 16;

const TAPE_LIMIT = 64;
const HISTORY_LIMIT = 900;

export class Simulation {
  readonly tape: Trade[] = [];
  /** Ring buffer of per-frame samples for the chart, oldest first. */
  readonly history: HistoryPoint[] = [];

  running = true;
  speed: Speed = 4;

  volume = 0;
  tradeCount = 0;
  messageCount = 0;

  private flow: MarketFlow;
  private lastMid: number | null = null;
  private frameTrade: { price: number; side: Side } | null = null;
  private listeners = new Set<() => void>();

  constructor(private engine: Engine) {
    this.flow = new MarketFlow(engine, DEFAULT_MARKET);
    this.ingest(this.flow.seedBook());
  }

  /** Advance one animation frame's worth of simulated flow, then notify. */
  frame(): void {
    if (this.running) {
      for (let i = 0; i < this.speed; i++) {
        this.ingest(this.flow.step());
      }
    }
    this.sampleHistory();
    for (const fn of this.listeners) fn();
  }

  subscribe(fn: () => void): () => void {
    this.listeners.add(fn);
    return () => this.listeners.delete(fn);
  }

  /** Manual order entry from the ticket panel. */
  submitUser(side: Side, price: number | null, qty: number, tif: Tif): EngineEvent[] {
    const submission =
      price === null
        ? this.engine.submitMarket(side, qty, this.flow.time)
        : this.engine.submitLimit(side, price, qty, tif, this.flow.time);
    this.ingest(submission.events);
    return submission.events;
  }

  get bestBid(): number | null {
    return this.engine.bestBid();
  }

  get bestAsk(): number | null {
    return this.engine.bestAsk();
  }

  get openOrders(): number {
    return this.engine.openOrders();
  }

  depth(side: Side, levels: number) {
    return this.engine.depth(side, levels);
  }

  private ingest(events: EngineEvent[]): void {
    this.messageCount += events.length;
    for (const event of events) {
      if (event.type !== "trade") continue;
      this.tradeCount++;
      this.volume += event.qty;
      this.frameTrade = { price: event.price, side: event.takerSide };
      this.tape.unshift({
        seq: event.seq,
        price: event.price,
        qty: event.qty,
        takerSide: event.takerSide,
        timestamp: event.timestamp,
      });
    }
    if (this.tape.length > TAPE_LIMIT) this.tape.length = TAPE_LIMIT;
  }

  private sampleHistory(): void {
    const bid = this.engine.bestBid();
    const ask = this.engine.bestAsk();
    if (bid !== null && ask !== null) this.lastMid = (bid + ask) / 2;
    this.history.push({ mid: this.lastMid, trade: this.frameTrade });
    this.frameTrade = null;
    if (this.history.length > HISTORY_LIMIT) this.history.splice(0, this.history.length - HISTORY_LIMIT);
  }
}
