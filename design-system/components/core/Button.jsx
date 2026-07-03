// Rustloader Button — variants map to the Iced theme's PrimaryButton /
// SecondaryButton / DestructiveButton / IconButton(ghost) styles, restyled
// to the dark rust/copper system.
import React, { useState } from "react";
import { Icon } from "./Icon.jsx";

const SIZES = {
  sm: { padding: "5px 10px", fontSize: "var(--text-sm)", gap: 6, iconSize: 13 },
  md: { padding: "8px 16px", fontSize: "var(--text-md)", gap: 7, iconSize: 15 },
  lg: { padding: "11px 22px", fontSize: "var(--text-lg)", gap: 8, iconSize: 16 },
};

export function Button({ variant = "secondary", size = "md", icon, children, disabled, onClick, style, title }) {
  const [hover, setHover] = useState(false);
  const [active, setActive] = useState(false);
  const s = SIZES[size] || SIZES.md;

  const variants = {
    primary: {
      background: active ? "var(--accent-pressed)" : hover ? "var(--accent-hover)" : "var(--accent)",
      color: "var(--fg-on-accent)",
      border: "1px solid transparent",
      boxShadow: hover && !active ? "var(--shadow-accent)" : "none",
    },
    secondary: {
      background: hover ? "var(--surface-hover)" : "var(--surface-card)",
      color: "var(--text-body)",
      border: "1px solid var(--border-strong)",
      boxShadow: "var(--inset-highlight)",
    },
    ghost: {
      background: hover ? "var(--surface-hover)" : "transparent",
      color: hover ? "var(--text-body)" : "var(--text-muted)",
      border: "1px solid transparent",
    },
    destructive: {
      background: hover ? "var(--bg-danger-soft)" : "transparent",
      color: "var(--danger-400)",
      border: "1px solid transparent",
    },
  };

  const v = variants[variant] || variants.secondary;

  return (
    <button
      type="button"
      title={title}
      disabled={disabled}
      onClick={onClick}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => { setHover(false); setActive(false); }}
      onMouseDown={() => setActive(true)}
      onMouseUp={() => setActive(false)}
      style={{
        display: "inline-flex", alignItems: "center", justifyContent: "center", gap: s.gap,
        padding: s.padding, fontSize: s.fontSize, fontFamily: "var(--font-ui)", fontWeight: 500,
        borderRadius: "var(--radius-md)", cursor: disabled ? "default" : "pointer",
        transition: "background var(--dur-fast) var(--ease-out), box-shadow var(--dur-fast) var(--ease-out)",
        outline: "none",
        ...(disabled ? { background: "var(--surface-card)", color: "var(--fg-disabled)", border: "1px solid var(--border-hairline)", boxShadow: "none" } : v),
        ...style,
      }}
      onFocus={(e) => { e.target.style.boxShadow = "var(--focus-ring)"; }}
      onBlur={(e) => { e.target.style.boxShadow = v.boxShadow || "none"; }}
    >
      {icon ? <Icon name={icon} size={s.iconSize} /> : null}
      {children}
    </button>
  );
}
