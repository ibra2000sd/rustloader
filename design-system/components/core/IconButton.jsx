// Rustloader IconButton — square ghost control (per-item actions, header)
import React, { useState } from "react";
import { Icon } from "./Icon.jsx";

export function IconButton({ icon, size = 28, iconSize = 15, danger, title, onClick, disabled, style }) {
  const [hover, setHover] = useState(false);
  return (
    <button
      type="button"
      title={title}
      aria-label={title}
      disabled={disabled}
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      style={{
        width: size, height: size, display: "inline-flex", alignItems: "center", justifyContent: "center",
        background: hover && !disabled ? (danger ? "var(--bg-danger-soft)" : "var(--surface-hover)") : "transparent",
        color: disabled ? "var(--fg-disabled)" : danger ? "var(--danger-400)" : hover ? "var(--text-body)" : "var(--text-muted)",
        border: "none", borderRadius: "var(--radius-sm)", cursor: disabled ? "default" : "pointer",
        transition: "background var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out)",
        outline: "none", padding: 0,
        ...style,
      }}
      onFocus={(e) => { e.target.style.boxShadow = "var(--focus-ring)"; }}
      onBlur={(e) => { e.target.style.boxShadow = "none"; }}
    >
      <Icon name={icon} size={iconSize} />
    </button>
  );
}
