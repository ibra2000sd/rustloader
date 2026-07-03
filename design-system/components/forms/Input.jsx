// Rustloader Input — text field (URL bar uses UrlBar in downloads/)
import React, { useState } from "react";

export function Input({ value, onChange, placeholder, error, mono = false, disabled, style, ...rest }) {
  const [focus, setFocus] = useState(false);
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange && onChange(e.target.value)}
      placeholder={placeholder}
      disabled={disabled}
      onFocus={() => setFocus(true)}
      onBlur={() => setFocus(false)}
      style={{
        fontFamily: mono ? "var(--font-mono)" : "var(--font-ui)",
        fontSize: "var(--text-md)", color: disabled ? "var(--fg-disabled)" : "var(--text-body)",
        background: "var(--bg-input)",
        border: `1px solid ${error ? "var(--danger-400)" : focus ? "var(--border-accent)" : "var(--border-strong)"}`,
        borderRadius: "var(--radius-md)", padding: "9px 12px",
        outline: "none", boxSizing: "border-box", width: "100%",
        boxShadow: focus && !error ? "0 0 0 3px var(--bg-accent-soft)" : "var(--inset-highlight)",
        transition: "border-color var(--dur-fast) var(--ease-out), box-shadow var(--dur-fast) var(--ease-out)",
        ...style,
      }}
      {...rest}
    />
  );
}
