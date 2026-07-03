// Rustloader SegmentBar — the signature visual: N parallel byte-range
// segments filling independently, reading as one machined bar.
import React from "react";

export function SegmentBar({ segments = [], count, status = "downloading", height = 14, gap = 2, style }) {
  // Either pass `segments` (array of 0–100 fills) or `count` for an empty/queued bar
  const cells = segments.length ? segments : Array.from({ length: count || 16 }, () => 0);
  const colors = {
    downloading: "var(--accent)",
    complete: "var(--success-500)",
    paused: "var(--warning-400)",
    stalled: "var(--warning-400)",
    error: "var(--danger-400)",
    queued: "var(--fg-disabled)",
  };
  const fill = colors[status] || colors.downloading;
  return (
    <div style={{ display: "flex", gap, width: "100%", ...style }} aria-hidden="true">
      {cells.map((pct, i) => {
        const p = Math.max(0, Math.min(100, pct));
        const active = status === "downloading" && p > 0 && p < 100;
        return (
          <div key={i} style={{
            flex: 1, height, background: "var(--progress-track)",
            borderRadius: 2, overflow: "hidden", position: "relative",
          }}>
            <div style={{
              position: "absolute", inset: 0, width: `${p}%`,
              background: p >= 100 && status === "downloading" ? "var(--rust-600)" : fill,
              opacity: status === "paused" || status === "queued" ? 0.55 : 1,
              animation: status === "stalled" ? "rl-pulse 2s var(--ease-in-out) infinite" : "none",
              transition: "width var(--dur-med) var(--ease-out)",
              boxShadow: active ? "0 0 8px rgba(207,111,56,0.5)" : "none", // web-only glow
            }}></div>
          </div>
        );
      })}
    </div>
  );
}
