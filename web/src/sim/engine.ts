/**
 * Typed facade over the Rust engine's WASM bridge.
 *
 * The generated bindings live in `src/wasm/` (output of
 * `wasm-pack build --target web`). This module is the only file that touches
 * them, so the rest of the app depends on these types, not on wasm-bindgen's
 * output shape.
 */
import init, { Exchange } from "../wasm/lob_wasm";

export type Side = "bid" | "ask";
export type Tif = "gtc" | "ioc" | "fok";

export type EngineEvent =
  | { type: "accepted"; seq: number; id: number; side: Side; price: number | null; qty: number; timestamp: number }
  | { type: "rejected"; seq: number; id: number; reason: string; timestamp: number }
  | {
      type: "trade";
      seq: number;
      makerId: number;
      takerId: number;
      takerSide: Side;
      price: number;
      qty: number;
      makerRemaining: number;
      timestamp: number;
    }
  | { type: "posted"; seq: number; id: number; side: Side; price: number; qty: number; timestamp: number }
  | { type: "cancelled"; seq: number; id: number; reason: string; qty: number; timestamp: number };

export interface Submission {
  id: number;
  events: EngineEvent[];
}

/** One aggregated depth level decoded from the flat snapshot buffer. */
export interface DepthLevel {
  price: number;
  qty: number;
  orders: number;
}

export interface Engine {
  submitLimit(side: Side, price: number, qty: number, tif: Tif, timestamp: number): Submission;
  submitMarket(side: Side, qty: number, timestamp: number): Submission;
  cancel(id: number, timestamp: number): EngineEvent | null;
  depth(side: Side, levels: number): DepthLevel[];
  bestBid(): number | null;
  bestAsk(): number | null;
  openOrders(): number;
}

class WasmEngine implements Engine {
  constructor(private exchange: Exchange) {}

  submitLimit(side: Side, price: number, qty: number, tif: Tif, timestamp: number): Submission {
    const sub = this.exchange.submitLimit(side, price, qty, tif, timestamp);
    return { id: sub.id, events: sub.events as EngineEvent[] };
  }

  submitMarket(side: Side, qty: number, timestamp: number): Submission {
    const sub = this.exchange.submitMarket(side, qty, timestamp);
    return { id: sub.id, events: sub.events as EngineEvent[] };
  }

  cancel(id: number, timestamp: number): EngineEvent | null {
    return this.exchange.cancel(id, timestamp) as EngineEvent | null;
  }

  depth(side: Side, levels: number): DepthLevel[] {
    const flat = this.exchange.depth(side, levels);
    const out: DepthLevel[] = [];
    for (let i = 0; i + 2 < flat.length; i += 3) {
      out.push({ price: flat[i], qty: flat[i + 1], orders: flat[i + 2] });
    }
    return out;
  }

  bestBid(): number | null {
    return this.exchange.bestBid() ?? null;
  }

  bestAsk(): number | null {
    return this.exchange.bestAsk() ?? null;
  }

  openOrders(): number {
    return this.exchange.openOrders();
  }
}

/** Initialise the WASM module (idempotent) and construct a fresh engine. */
let wasmReady: Promise<unknown> | null = null;

export async function createEngine(): Promise<Engine> {
  wasmReady ??= init();
  await wasmReady;
  return new WasmEngine(new Exchange());
}
