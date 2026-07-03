// Rustloader Toggle — opt-in switches (clipboard monitoring), default off
import React from "react";

export function Toggle({ checked = false, onChange, label, disabled, style }) {
  return (
    <label style={{ display: "inline-flex", alignItems: "center", gap: 10, cursor: disabled ? "default" : "pointer", ...style }}>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange && onChange(!checked)}
        style={{
          width: 36, height: 20, borderRadius: 999, border: "1px solid var(--border-strong)",
          background: checked ? "var(--accent)" : "var(--progress-track)",
          position: "relative", cursor: "inherit", padding: 0, outline: "none", flexShrink: 0,
          transition: "background var(--dur-med) var(--ease-out)",
          opacity: disabled ? 0.5 : 1,
        }}
        onFocus={(e) => { e.target.style.boxShadow = "var(--focus-ring)"; }}
        onBlur={(e) => { e.target.style.boxShadow = "none"; }}
      >
        <span style={{
          position: "absolute", top: 2, left: checked ? 17 : 2, width: 14, height: 14,
          borderRadius: 999, background: checked ? "var(--fg-on-accent)" : "var(--fg-2)",
          transition: "left var(--dur-med) var(--ease-out), background var(--dur-med) var(--ease-out)",
        }}></span>
      </button>
      {label ? <span style={{ fontFamily: "var(--font-ui)", fontSize: "var(--text-md)", color: disabled ? "var(--fg-disabled)" : "var(--text-body)" }}>{label}</span> : null}
    </label>
  );
}
