// Rustloader Select — quality/format/cookies pick-list
import React, { useState } from "react";
import { Icon } from "../core/Icon.jsx";

export function Select({ options = [], value, onChange, label, width, disabled, style }) {
  const [focus, setFocus] = useState(false);
  return (
    <label style={{ display: "inline-flex", flexDirection: "column", gap: 4, width, ...style }}>
      {label ? (
        <span style={{
          fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500,
          letterSpacing: "var(--tracking-caps)", textTransform: "uppercase", color: "var(--text-label)",
        }}>{label}</span>
      ) : null}
      <span style={{ position: "relative", display: "inline-flex", alignItems: "center" }}>
        <select
          value={value}
          disabled={disabled}
          onChange={(e) => onChange && onChange(e.target.value)}
          onFocus={() => setFocus(true)}
          onBlur={() => setFocus(false)}
          style={{
            appearance: "none", WebkitAppearance: "none", width: "100%",
            fontFamily: "var(--font-ui)", fontSize: "var(--text-sm)", fontWeight: 500,
            color: disabled ? "var(--fg-disabled)" : "var(--text-body)",
            background: "var(--bg-input)",
            border: `1px solid ${focus ? "var(--border-accent)" : "var(--border-strong)"}`,
            borderRadius: "var(--radius-md)", padding: "7px 30px 7px 10px",
            outline: "none", cursor: disabled ? "default" : "pointer",
            boxShadow: focus ? "0 0 0 3px var(--bg-accent-soft)" : "var(--inset-highlight)",
            transition: "border-color var(--dur-fast) var(--ease-out)",
          }}
        >
          {options.map((o) => <option key={o} value={o}>{o}</option>)}
        </select>
        <span style={{ position: "absolute", right: 9, pointerEvents: "none", color: "var(--text-label)", display: "flex" }}>
          <Icon name="chevron-down" size={13} />
        </span>
      </span>
    </label>
  );
}
