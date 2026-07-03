// Rustloader Downloads (main) view — hero UrlBar, clipboard banner, active queue
import React from "react";
import { UrlBar } from "../../components/downloads/UrlBar.jsx";
import { ClipboardBanner } from "../../components/downloads/ClipboardBanner.jsx";
import { DownloadItem } from "../../components/downloads/DownloadItem.jsx";
import { Button } from "../../components/core/Button.jsx";
import { Icon } from "../../components/core/Icon.jsx";

export function DownloadsScreen({ state, actions }) {
  const { url, quality, format, extracting, banner, items } = state;
  const active = items.filter((i) => i.status !== "complete");
  const done = items.filter((i) => i.status === "complete");

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 14, height: "100%", minHeight: 0 }}>
      <UrlBar
        value={url} onChange={actions.setUrl}
        quality={quality} onQualityChange={actions.setQuality}
        format={format} onFormatChange={actions.setFormat}
        extracting={extracting}
        onDownload={actions.startDownload}
        onPaste={actions.pasteUrl}
      />
      {banner ? (
        <ClipboardBanner url={banner} onConfirm={actions.confirmBanner} onDismiss={actions.dismissBanner} disabled={extracting} />
      ) : null}

      {items.length === 0 ? (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 8 }}>
          <span style={{ color: "var(--text-label)", display: "flex" }}><Icon name="inbox" size={28} strokeWidth={1.5} /></span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "var(--text-lg)", fontWeight: 500, color: "var(--text-muted)" }}>No active downloads</span>
          <span style={{ fontFamily: "var(--font-ui)", fontSize: "var(--text-md)", color: "var(--text-label)" }}>Paste a URL above — or copy one anywhere and Rustloader will offer to queue it.</span>
        </div>
      ) : (
        <>
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", fontWeight: 500, letterSpacing: "var(--tracking-caps)", textTransform: "uppercase", color: "var(--text-label)" }}>
              Active queue · {active.length}
            </span>
            <span style={{ flex: 1 }}></span>
            <Button size="sm" icon="play" onClick={actions.resumeAll}>Resume All</Button>
            {done.length > 0 ? <Button size="sm" variant="ghost" onClick={actions.clearCompleted}>Clear Completed</Button> : null}
          </div>
          <div style={{ flex: 1, overflowY: "auto", display: "flex", flexDirection: "column", gap: 10, minHeight: 0, paddingRight: 2 }}>
            {items.map((item) => (
              <DownloadItem
                key={item.id}
                {...item}
                on={{
                  pause: () => actions.setStatus(item.id, "paused"),
                  resume: () => actions.setStatus(item.id, "downloading"),
                  restart: () => actions.setStatus(item.id, "downloading"),
                  retry: () => actions.setStatus(item.id, "downloading"),
                  cancel: () => actions.remove(item.id),
                  remove: () => actions.remove(item.id),
                  openFolder: () => {},
                }}
              />
            ))}
          </div>
        </>
      )}
    </div>
  );
}
