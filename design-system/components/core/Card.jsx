// Rustloader Card — machined panel: card surface + hairline + inset top-highlight
import React from "react";

export function Card({ elevation = "card", glass = false, accent = false, padding = 16, children, style }) {
  const bg = glass ? "var(--bg-glass)" : elevation === "panel" ? "var(--surface-panel)" : "var(--surface-card)";
  return (
    <div
      style={{
        background: bg,
        backdropFilter: glass ? "var(--glass-blur)" : undefined, // web-only; Iced falls back to solid
        border: `1px solid ${accent ? "var(--border-accent)" : "var(--border-hairline)"}`,
        borderRadius: elevation === "panel" ? "var(--radius-xl)" : "var(--radius-lg)",
        boxShadow: glass ? "var(--shadow-3)" : "var(--shadow-1), var(--inset-highlight)",
        padding,
        boxSizing: "border-box",
        ...style,
      }}
    >
      {children}
    </div>
  );
}
