import { useEffect, useRef } from "react";
import type { Simulation } from "../sim/simulation";

/**
 * A DPR-aware canvas that repaints from the simulation on every frame
 * notification, sized by ResizeObserver. `draw` receives a context whose
 * coordinate space is CSS pixels regardless of device pixel ratio.
 */
export function useCanvas(sim: Simulation, draw: (ctx: CanvasRenderingContext2D, w: number, h: number) => void) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const drawRef = useRef(draw);
  drawRef.current = draw;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let width = 0;
    let height = 0;

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      width = rect.width;
      height = rect.height;
      canvas.width = Math.round(rect.width * dpr);
      canvas.height = Math.round(rect.height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    resize();

    const unsubscribe = sim.subscribe(() => {
      if (width > 0 && height > 0) drawRef.current(ctx, width, height);
    });

    return () => {
      observer.disconnect();
      unsubscribe();
    };
  }, [sim]);

  return canvasRef;
}
