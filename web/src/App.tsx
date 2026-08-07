import { useEffect, useMemo, useReducer, useState } from "react";
import { createEngine } from "./sim/engine";
import { Simulation, type Speed } from "./sim/simulation";
import { DepthLadder, fmtPrice } from "./components/DepthLadder";
import { PriceChart } from "./components/PriceChart";
import { Tape } from "./components/Tape";
import { OrderTicket } from "./components/OrderTicket";

/** Re-render React chrome at ~8 Hz off the sim's frame notifications. */
function useSimChrome(sim: Simulation | null) {
  const [, force] = useReducer((n: number) => n + 1, 0);
  useEffect(() => {
    if (!sim) return;
    let last = 0;
    return sim.subscribe(() => {
      const now = performance.now();
      if (now - last > 125) {
        last = now;
        force();
      }
    });
  }, [sim]);
}

export default function App() {
  const [sim, setSim] = useState<Simulation | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    createEngine()
      .then((engine) => {
        if (!cancelled) setSim(new Simulation(engine));
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
    return () => {
      cancelled = true;
    };
  }, []);

  // One rAF loop drives the whole app.
  useEffect(() => {
    if (!sim) return;
    let raf = 0;
    const loop = () => {
      sim.frame();
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [sim]);

  useSimChrome(sim);

  const speeds = useMemo<Speed[]>(() => [1, 4, 16], []);

  if (error) {
    return (
      <div className="app" style={{ placeItems: "center", display: "grid" }}>
        <p role="alert">engine failed to load: {error}</p>
      </div>
    );
  }

  if (!sim) {
    return (
      <div className="app" style={{ placeItems: "center", display: "grid" }}>
        <p>loading engine…</p>
      </div>
    );
  }

  const bid = sim.bestBid;
  const ask = sim.bestAsk;

  return (
    <div className="app">
      <header className="topbar">
        <span className="wordmark">LEDGER</span>
        <span className="instrument">LDGR · synthetic · tick 0.01</span>
        <div className="bbo">
          <span>
            <span className="label">BID</span>
            <span className="bid-px">{bid !== null ? fmtPrice(bid) : "-"}</span>
          </span>
          <span>
            <span className="label">ASK</span>
            <span className="ask-px">{ask !== null ? fmtPrice(ask) : "-"}</span>
          </span>
          <span>
            <span className="label">SPR</span>
            {bid !== null && ask !== null ? fmtPrice(ask - bid) : "-"}
          </span>
        </div>
        <div className="controls">
          <div className="speed" role="group" aria-label="Simulation speed">
            {speeds.map((s) => (
              <button
                key={s}
                aria-pressed={sim.speed === s}
                onClick={() => {
                  sim.speed = s;
                }}
              >
                {s}×
              </button>
            ))}
          </div>
          <button
            className="btn"
            aria-pressed={!sim.running}
            onClick={() => {
              sim.running = !sim.running;
            }}
          >
            {sim.running ? "PAUSE" : "RESUME"}
          </button>
        </div>
      </header>

      <main className="workspace">
        <section className="pane" aria-label="Order book">
          <div className="pane-title">
            <span>Book</span>
            <span>{sim.openOrders} open</span>
          </div>
          <div className="pane-body">
            <DepthLadder sim={sim} />
          </div>
        </section>

        <section className="pane" aria-label="Price chart">
          <div className="pane-title">
            <span>Mid + prints</span>
          </div>
          <div className="pane-body">
            <PriceChart sim={sim} />
          </div>
        </section>

        <section className="pane" aria-label="Time and sales">
          <div className="pane-title">
            <span>Tape</span>
          </div>
          <div className="pane-body" style={{ overflow: "hidden" }}>
            <Tape sim={sim} />
          </div>
          <OrderTicket sim={sim} />
        </section>
      </main>

      <footer className="statusbar">
        <span>
          vol <b>{sim.volume.toLocaleString()}</b>
        </span>
        <span>
          trades <b>{sim.tradeCount.toLocaleString()}</b>
        </span>
        <span>
          msgs <b>{sim.messageCount.toLocaleString()}</b>
        </span>
        <span className="engine">
          <span className="dot" aria-hidden="true" />
          rust · wasm · deterministic seed
        </span>
      </footer>
    </div>
  );
}
