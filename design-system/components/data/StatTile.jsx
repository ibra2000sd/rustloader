// Rustloader StatTile — header global stats (bento tile)
import React from "react";
import { Icon } from "../core/Icon.jsx";

export function StatTile({ label, value, unit, icon, hot = false, style }) {
  return (
    <div style={{
      display: "flex", flexDirection: "column", gap: 3,
      background: "var(--surface-card)", border: "1px solid var(--border-hairline)",
      borderRadius: "var(--radius-lg)", padding: "10px 14px",
      boxShadow: "var(--shadow-1), var(--inset-highlight)", minWidth: 110, boxSizing: "border-box",
      ...style,
    }}>
      <span style={{
        display: "flex", alignItems: "center", gap: 5,
        fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500,
        letterSpacing: "var(--tracking-caps)", textTransform: "uppercase", color: "var(--text-label)",
      }}>
        {icon ? <Icon name={icon} size={11} /> : null}{label}
      </span>
      <span style={{
        fontFamily: "var(--font-mono)", fontSize: "var(--text-xl)", fontWeight: 600,
        color: hot ? "var(--data-hot)" : "var(--text-body)", lineHeight: 1.15,
      }}>
        {value}{unit ? <span style={{ fontSize: "var(--text-xs)", fontWeight: 500, color: "var(--text-muted)", marginLeft: 4 }}>{unit}</span> : null}
      </span>
    </div>
  );
}
