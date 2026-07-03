// Rustloader History view — persisted downloads, read/delete only
import React from "react";
import { HistoryItem } from "../../components/downloads/HistoryItem.jsx";
import { Button } from "../../components/core/Button.jsx";
import { Icon } from "../../components/core/Icon.jsx";

export function HistoryScreen({ records, onRemove, onRedownload }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14, height: "100%", minHeight: 0 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500, letterSpacing: "var(--tracking-caps)", textTransform: "uppercase", color: "var(--text-label)" }}>
          {records.length} downloads
        </span>
        <span style={{ flex: 1 }}></span>
        <Button size="sm" icon="rotate-cw">Refresh</Button>
      </div>
      {records.length === 0 ? (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 8 }}>
          <span style={{ color: "var(--text-label)", display: "flex" }}><Icon name="history" size={28} strokeWidth={1.5} /></span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "var(--text-lg)", fontWeight: 500, color: "var(--text-muted)" }}>No download history yet</span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "var(--text-md)", color: "var(--text-label)" }}>Downloads you complete will show up here.</span>
        </div>
      ) : (
        <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 8, minHeight: 0, paddingRight: 2 }}>
          {records.map((r) => (
            <HistoryItem key={r.id} {...r} on={{ remove: () => onRemove(r.id), redownload: () => onRedownload(r), openFolder: () => {} }} />
          ))}
        </div>
      )}
    </div>
  );
}
