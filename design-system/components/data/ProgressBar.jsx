// Rustloader ProgressBar — single-track progress with state colors
import React from "react";

const STATE_COLOR = {
  downloading: "var(--accent)",
  complete: "var(--success-500)",
  paused: "var(--warning-400)",
  stalled: "var(--warning-400)",
  error: "var(--danger-400)",
  queued: "var(--fg-disabled)",
};

export function ProgressBar({ value = 0, status = "downloading", height = 5, style }) {
  const color = STATE_COLOR[status] || STATE_COLOR.downloading;
  return (
    <div
      role="progressbar"
      aria-valuenow={Math.round(value)}
      aria-valuemin={0}
      aria-valuemax={100}
      style={{
        height, background: "var(--progress-track)", borderRadius: "var(--radius-xs)",
        overflow: "hidden", width: "100%", ...style,
      }}
    >
      <div style={{
        height: "100%", width: `${Math.max(0, Math.min(100, value))}%`,
        background: color, borderRadius: "var(--radius-xs)",
        opacity: status === "paused" || status === "queued" ? 0.55 : 1,
        animation: status === "stalled" ? "rl-pulse 2s var(--ease-in-out) infinite" : "none",
        transition: "width var(--dur-med) var(--ease-out), background var(--dur-med) var(--ease-out)",
      }}></div>
    </div>
  );
}
