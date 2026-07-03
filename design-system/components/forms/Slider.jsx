// Rustloader Slider — segments / concurrency, with live mono value
import React from "react";

export function Slider({ min = 0, max = 100, value = 0, onChange, label, unit = "", width, style }) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, width, ...style }}>
      {label !== undefined ? (
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
          <span style={{
            fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500,
            letterSpacing: "var(--tracking-caps)", textTransform: "uppercase", color: "var(--text-label)",
          }}>{label}</span>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-sm)", fontWeight: 500, color: "var(--data-hot)" }}>
            {value}{unit}
          </span>
        </div>
      ) : null}
      <input
        type="range"
        min={min} max={max} value={value}
        onChange={(e) => onChange && onChange(Number(e.target.value))}
        className="rl-slider"
        style={{
          appearance: "none", WebkitAppearance: "none", width: "100%", height: 4, margin: 0,
          borderRadius: "var(--radius-xs)", outline: "none", cursor: "pointer",
          background: `linear-gradient(to right, var(--accent) ${pct}%, var(--progress-track) ${pct}%)`,
        }}
      />
      <style>{`
        .rl-slider::-webkit-slider-thumb { -webkit-appearance: none; width: 14px; height: 14px; border-radius: 999px; background: var(--fg-1); border: 2px solid var(--accent); box-shadow: var(--shadow-1); }
        .rl-slider:focus-visible { box-shadow: var(--focus-ring); }
      `}</style>
    </div>
  );
}
