import type { Simulation } from "../sim/simulation";
import { useCanvas } from "./useCanvas";

const ROW = 21;
const COLORS = {
  bid: "#34d399",
  ask: "#fbbf24",
  bidDim: "rgba(52, 211, 153, 0.16)",
  askDim: "rgba(251, 191, 36, 0.13)",
  text: "#dde3ec",
  muted: "#6b7484",
  grid: "#1a1f28",
};

/** Format a tick price as a decimal (tick = 1/100). */
export function fmtPrice(ticks: number): string {
  return (ticks / 100).toFixed(2);
}

/**
 * The order-book ladder. Asks stack upward from the spread row, bids stack
 * downward; bar length is level quantity relative to the largest visible
 * level, so relative depth reads instantly.
 */
export function DepthLadder({ sim }: { sim: Simulation }) {
  const ref = useCanvas(sim, (ctx, w, h) => {
    ctx.clearRect(0, 0, w, h);
    ctx.font = "12px 'JetBrains Mono', 'Cascadia Code', Consolas, monospace";
    ctx.textBaseline = "middle";

    const levelsEachSide = Math.floor((h / ROW - 1) / 2);
    const bids = sim.depth("bid", levelsEachSide);
    const asks = sim.depth("ask", levelsEachSide);
    const maxQty = Math.max(1, ...bids.map((l) => l.qty), ...asks.map((l) => l.qty));

    const mid = h / 2;
    const barMax = w * 0.55;
    const qtyX = w - 12;
    const priceX = 12;

    // Spread row.
    const bb = sim.bestBid;
    const ba = sim.bestAsk;
    ctx.fillStyle = COLORS.muted;
    ctx.textAlign = "center";
    const spreadLabel = bb !== null && ba !== null ? `spread ${fmtPrice(ba - bb)}` : "-";
    ctx.fillText(spreadLabel, w / 2, mid);
    ctx.strokeStyle = COLORS.grid;
    ctx.beginPath();
    ctx.moveTo(0, mid - ROW / 2);
    ctx.lineTo(w, mid - ROW / 2);
    ctx.moveTo(0, mid + ROW / 2);
    ctx.lineTo(w, mid + ROW / 2);
    ctx.stroke();

    // Asks go up from the spread row, best ask nearest the middle.
    asks.forEach((level, i) => {
      const y = mid - ROW * (i + 1);
      if (y < ROW / 2) return;
      const barW = (level.qty / maxQty) * barMax;
      ctx.fillStyle = COLORS.askDim;
      ctx.fillRect(w - barW, y - ROW / 2 + 2, barW, ROW - 4);
      ctx.fillStyle = COLORS.ask;
      ctx.textAlign = "left";
      ctx.fillText(fmtPrice(level.price), priceX, y);
      ctx.fillStyle = COLORS.muted;
      ctx.textAlign = "right";
      ctx.fillText(String(level.qty), qtyX, y);
    });

    // Bids go down, best bid nearest the middle.
    bids.forEach((level, i) => {
      const y = mid + ROW * (i + 1);
      if (y > h - ROW / 2) return;
      const barW = (level.qty / maxQty) * barMax;
      ctx.fillStyle = COLORS.bidDim;
      ctx.fillRect(w - barW, y - ROW / 2 + 2, barW, ROW - 4);
      ctx.fillStyle = COLORS.bid;
      ctx.textAlign = "left";
      ctx.fillText(fmtPrice(level.price), priceX, y);
      ctx.fillStyle = COLORS.muted;
      ctx.textAlign = "right";
      ctx.fillText(String(level.qty), qtyX, y);
    });
  });

  return <canvas ref={ref} role="img" aria-label="Order book depth ladder" />;
}
