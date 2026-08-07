import { useState } from "react";
import type { Side, Tif } from "../sim/engine";
import type { Simulation } from "../sim/simulation";

type OrderKind = "limit" | "market";

/**
 * Manual order entry. Orders submitted here go through the same Rust
 * matching path as the simulated flow, the user is just one more
 * participant in the market.
 */
export function OrderTicket({ sim }: { sim: Simulation }) {
  const [side, setSide] = useState<Side>("bid");
  const [kind, setKind] = useState<OrderKind>("limit");
  const [tif, setTif] = useState<Tif>("gtc");
  const [price, setPrice] = useState("");
  const [qty, setQty] = useState("25");
  const [report, setReport] = useState<string | null>(null);

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    const lots = Number.parseInt(qty, 10);
    if (!Number.isFinite(lots) || lots <= 0) {
      setReport("qty must be a positive integer");
      return;
    }

    let ticks: number | null = null;
    if (kind === "limit") {
      const parsed = Number.parseFloat(price);
      if (!Number.isFinite(parsed)) {
        setReport("price required for limit orders");
        return;
      }
      ticks = Math.round(parsed * 100);
    }

    const events = sim.submitUser(side, ticks, lots, tif);
    const filled = events
      .filter((ev) => ev.type === "trade")
      .reduce((sum, ev) => sum + (ev.type === "trade" ? ev.qty : 0), 0);
    const posted = events.some((ev) => ev.type === "posted");
    const rejected = events.find((ev) => ev.type === "rejected");

    if (rejected && rejected.type === "rejected") setReport(`rejected: ${rejected.reason}`);
    else if (filled > 0 && posted) setReport(`filled ${filled}, rest posted`);
    else if (filled > 0) setReport(`filled ${filled}`);
    else if (posted) setReport("posted to book");
    else setReport("cancelled (nothing crossed)");
  };

  return (
    <form className="ticket" onSubmit={submit} aria-label="Order entry">
      <div className="field" style={{ gridColumn: "1 / -1" }}>
        <label id="side-label">Side</label>
        <div className="side-toggle" role="group" aria-labelledby="side-label">
          <button type="button" className="bid" aria-pressed={side === "bid"} onClick={() => setSide("bid")}>
            BUY
          </button>
          <button type="button" className="ask" aria-pressed={side === "ask"} onClick={() => setSide("ask")}>
            SELL
          </button>
        </div>
      </div>

      <div className="field">
        <label htmlFor="ot-kind">Type</label>
        <select id="ot-kind" value={kind} onChange={(e) => setKind(e.target.value as OrderKind)}>
          <option value="limit">limit</option>
          <option value="market">market</option>
        </select>
      </div>

      <div className="field">
        <label htmlFor="ot-tif">TIF</label>
        <select id="ot-tif" value={tif} onChange={(e) => setTif(e.target.value as Tif)} disabled={kind === "market"}>
          <option value="gtc">GTC</option>
          <option value="ioc">IOC</option>
          <option value="fok">FOK</option>
        </select>
      </div>

      <div className="field">
        <label htmlFor="ot-price">Price</label>
        <input
          id="ot-price"
          inputMode="decimal"
          placeholder={kind === "market" ? "-" : "100.00"}
          value={kind === "market" ? "" : price}
          onChange={(e) => setPrice(e.target.value)}
          disabled={kind === "market"}
        />
      </div>

      <div className="field">
        <label htmlFor="ot-qty">Qty</label>
        <input id="ot-qty" inputMode="numeric" value={qty} onChange={(e) => setQty(e.target.value)} />
      </div>

      <button type="submit" className={`submit ${side}`}>
        {side === "bid" ? "BUY" : "SELL"} {kind.toUpperCase()}
      </button>

      {report && (
        <div className="field" style={{ gridColumn: "1 / -1" }} role="status">
          <label>Last report</label>
          <span>{report}</span>
        </div>
      )}
    </form>
  );
}
