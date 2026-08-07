import type { Simulation } from "../sim/simulation";
import { useCanvas } from "./useCanvas";
import { fmtPrice } from "./DepthLadder";

const COLORS = {
  mid: "#dde3ec",
  bid: "#34d399",
  ask: "#fbbf24",
  grid: "#141922",
  muted: "#6b7484",
};

/**
 * Mid-price line with trade prints. Buys (bid takers) print emerald above
 * the tape's noise, sells print amber, the same encoding as the ladder, so
 * the two panes read as one instrument.
 */
export function PriceChart({ sim }: { sim: Simulation }) {
  const ref = useCanvas(sim, (ctx, w, h) => {
    ctx.clearRect(0, 0, w, h);

    const points = sim.history;
    const mids = points.map((p) => p.mid).filter((m): m is number => m !== null);
    if (mids.length < 2) return;

    let lo = Math.min(...mids);
    let hi = Math.max(...mids);
    for (const p of points) {
      if (p.trade) {
        lo = Math.min(lo, p.trade.price);
        hi = Math.max(hi, p.trade.price);
      }
    }
    const pad = Math.max(2, (hi - lo) * 0.12);
    lo -= pad;
    hi += pad;

    const x = (i: number) => (i / (points.length - 1)) * (w - 64);
    const y = (price: number) => h - ((price - lo) / (hi - lo)) * (h - 24) - 12;

    // Horizontal gridlines at round prices.
    ctx.strokeStyle = COLORS.grid;
    ctx.fillStyle = COLORS.muted;
    ctx.font = "11px 'JetBrains Mono', 'Cascadia Code', Consolas, monospace";
    ctx.textAlign = "left";
    ctx.textBaseline = "middle";
    const gridStep = Math.max(10, Math.round((hi - lo) / 5 / 10) * 10);
    for (let price = Math.ceil(lo / gridStep) * gridStep; price <= hi; price += gridStep) {
      const gy = y(price);
      ctx.beginPath();
      ctx.moveTo(0, gy);
      ctx.lineTo(w - 64, gy);
      ctx.stroke();
      ctx.fillText(fmtPrice(price), w - 56, gy);
    }

    // Trade prints under the line.
    for (let i = 0; i < points.length; i++) {
      const trade = points[i].trade;
      if (!trade) continue;
      ctx.fillStyle = trade.side === "bid" ? COLORS.bid : COLORS.ask;
      ctx.globalAlpha = 0.75;
      ctx.fillRect(x(i) - 1, y(trade.price) - 1, 3, 3);
      ctx.globalAlpha = 1;
    }

    // Mid-price line.
    ctx.strokeStyle = COLORS.mid;
    ctx.lineWidth = 1.25;
    ctx.beginPath();
    let started = false;
    for (let i = 0; i < points.length; i++) {
      const mid = points[i].mid;
      if (mid === null) continue;
      if (!started) {
        ctx.moveTo(x(i), y(mid));
        started = true;
      } else {
        ctx.lineTo(x(i), y(mid));
      }
    }
    ctx.stroke();
    ctx.lineWidth = 1;

    // Emphasised endpoint + last-mid label.
    const lastMid = mids[mids.length - 1];
    const ex = x(points.length - 1);
    const ey = y(lastMid);
    ctx.fillStyle = COLORS.mid;
    ctx.beginPath();
    ctx.arc(ex, ey, 2.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillText(fmtPrice(lastMid), w - 56, ey);
  });

  return <canvas ref={ref} role="img" aria-label="Mid-price chart with trade prints" />;
}
