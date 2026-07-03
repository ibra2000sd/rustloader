// Rustloader StatusBadge — download states with dot / pulse
import React from "react";

const STATES = {
  queued:      { color: "var(--text-muted)",   bg: "transparent",             label: "Queued" },
  downloading: { color: "var(--rust-300)",     bg: "var(--bg-accent-soft)",   label: "Downloading", pulse: true },
  paused:      { color: "var(--warning-400)",  bg: "var(--bg-warning-soft)",  label: "Paused" },
  stalled:     { color: "var(--warning-400)",  bg: "var(--bg-warning-soft)",  label: "Stalled", pulse: true },
  complete:    { color: "var(--success-400)",  bg: "var(--bg-success-soft)",  label: "Complete" },
  error:       { color: "var(--danger-400)",   bg: "var(--bg-danger-soft)",   label: "Failed" },
  extracting:  { color: "var(--text-muted)",   bg: "transparent",             label: "Extracting…", pulse: true },
};

export function StatusBadge({ status = "queued", label, style }) {
  const s = STATES[status] || STATES.queued;
  return (
    <span
      style={{
        display: "inline-flex", alignItems: "center", gap: 6,
        fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500,
        letterSpacing: "var(--tracking-caps)", textTransform: "uppercase",
        color: s.color, background: s.bg,
        border: `1px solid ${s.bg === "transparent" ? "var(--border-hairline)" : "transparent"}`,
        borderRadius: "var(--radius-xs)", padding: "3px 8px",
        ...style,
      }}
    >
      <span style={{
        width: 6, height: 6, borderRadius: 999, background: s.color,
        animation: s.pulse ? "rl-pulse 2s var(--ease-in-out) infinite" : "none",
      }}></span>
      {label || s.label}
    </span>
  );
}
