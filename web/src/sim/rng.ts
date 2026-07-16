/**
 * Deterministic PRNG (mulberry32). The whole simulation is seeded so a run
 * is reproducible: same seed, same order flow, same book — the browser-side
 * counterpart of the engine's replay determinism.
 */
export class Rng {
  private state: number;

  constructor(seed: number) {
    this.state = seed >>> 0;
  }

  /** Uniform float in [0, 1). */
  next(): number {
    this.state = (this.state + 0x6d2b79f5) | 0;
    let t = Math.imul(this.state ^ (this.state >>> 15), 1 | this.state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }

  /** Uniform integer in [min, max] inclusive. */
  int(min: number, max: number): number {
    return min + Math.floor(this.next() * (max - min + 1));
  }

  /** True with probability p. */
  chance(p: number): boolean {
    return this.next() < p;
  }

  /** Standard normal via Box–Muller. */
  gaussian(): number {
    const u = Math.max(this.next(), 1e-12);
    const v = this.next();
    return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
  }

  /** Geometric-ish positive integer with the given mean (≥ 1). */
  size(mean: number): number {
    return Math.max(1, Math.round(-Math.log(Math.max(this.next(), 1e-12)) * mean));
  }
}
