import type { Simulation } from "../sim/simulation";
import { fmtPrice } from "./DepthLadder";

/**
 * Time & sales. Rows are keyed by engine sequence number, so React reuses
 * DOM nodes as rows shift down. Color marks the aggressor: emerald = buyer
 * lifted the offer, amber = seller hit the bid.
 */
export function Tape({ sim }: { sim: Simulation }) {
  return (
    <div className="tape" role="log" aria-label="Time and sales">
      {sim.tape.slice(0, 40).map((trade) => (
        <div key={trade.seq} className={`tape-row ${trade.takerSide === "bid" ? "buy" : "sell"}`}>
          <span className="px">{fmtPrice(trade.price)}</span>
          <span className="qty">{trade.qty}</span>
          <span className="time">{trade.timestamp}</span>
        </div>
      ))}
    </div>
  );
}
