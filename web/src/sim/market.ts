/**
 * Synthetic order-flow generator.
 *
 * Not a price model, a *flow* model. Agents post limit orders around a
 * slowly wandering fair value, occasionally cross the spread with market
 * orders, and cancel stale quotes. Price emerges from the book, exactly as
 * it does on a real venue; nothing here sets the price directly.
 */
import type { Engine, EngineEvent } from "./engine";
import { Rng } from "./rng";

export interface MarketConfig {
  seed: number;
  /** Initial fair value, in ticks. */
  fairValue: number;
  /** Std-dev of the fair-value random walk, ticks per step. */
  drift: number;
  /** Mean resting-order size, lots. */
  meanSize: number;
  /** Probability a step sends an aggressive market order. */
  aggression: number;
  /** Probability a step cancels a random resting sim order. */
  churn: number;
  /** How many ticks wide agents quote around fair value. */
  quoteWidth: number;
}

export const DEFAULT_MARKET: MarketConfig = {
  seed: 0x1ed6e2, // "ledger", as close as hex allows
  fairValue: 10_000,
  drift: 0.35,
  meanSize: 18,
  aggression: 0.16,
  churn: 0.22,
  quoteWidth: 12,
};

export class MarketFlow {
  private rng: Rng;
  private fair: number;
  /** Ids of sim-owned resting orders, for churn cancels. */
  private resting: number[] = [];
  private clock = 0;

  constructor(
    private engine: Engine,
    private config: MarketConfig = DEFAULT_MARKET,
  ) {
    this.rng = new Rng(config.seed);
    this.fair = config.fairValue;
  }

  /** Pre-populate both sides so the book opens with realistic depth. */
  seedBook(): EngineEvent[] {
    const events: EngineEvent[] = [];
    for (let i = 0; i < 60; i++) {
      events.push(...this.quote());
    }
    return events;
  }

  /**
   * One simulation step: the fair value diffuses, then exactly one agent
   * acts (quote, aggress, or cancel). Returns the engine events produced.
   */
  step(): EngineEvent[] {
    this.clock++;
    this.fair += this.rng.gaussian() * this.config.drift;

    const roll = this.rng.next();
    if (roll < this.config.aggression) {
      return this.aggress();
    }
    if (roll < this.config.aggression + this.config.churn) {
      return this.cancelOne();
    }
    return this.quote();
  }

  get time(): number {
    return this.clock;
  }

  /** A passive agent posts a limit order near fair value. */
  private quote(): EngineEvent[] {
    const side = this.rng.chance(0.5) ? "bid" : "ask";
    // Offsets cluster near the touch and thin out with distance.
    const offset = 1 + Math.floor(Math.abs(this.rng.gaussian()) * this.config.quoteWidth * 0.45);
    const price = Math.round(side === "bid" ? this.fair - offset : this.fair + offset);
    const qty = this.rng.size(this.config.meanSize);

    const { id, events } = this.engine.submitLimit(side, price, qty, "gtc", this.clock);
    if (events.some((e) => e.type === "posted")) {
      this.resting.push(id);
      // Cap the tracked population so churn targets stay recent-ish.
      if (this.resting.length > 4000) this.resting.splice(0, 1000);
    }
    return events;
  }

  /** An impatient agent crosses the spread. */
  private aggress(): EngineEvent[] {
    const side = this.rng.chance(0.5) ? "bid" : "ask";
    const qty = this.rng.size(this.config.meanSize * 1.6);
    return this.engine.submitMarket(side, qty, this.clock).events;
  }

  /** A resting quote goes stale and is pulled. */
  private cancelOne(): EngineEvent[] {
    while (this.resting.length > 0) {
      const at = this.rng.int(0, this.resting.length - 1);
      const id = this.resting.splice(at, 1)[0];
      const event = this.engine.cancel(id, this.clock);
      if (event) return [event]; // may miss: order already filled
    }
    return [];
  }
}
