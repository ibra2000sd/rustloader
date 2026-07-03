// Rustloader AppShell — fixed 1080×720 window: icon rail, header with
// identity + global stats, view switching, simulated download engine.
import React, { useEffect, useState } from "react";
import { Icon } from "../../components/core/Icon.jsx";
import { IconButton } from "../../components/core/IconButton.jsx";
import { StatTile } from "../../components/data/StatTile.jsx";
import { DownloadsScreen } from "./DownloadsScreen.jsx";
import { HistoryScreen } from "./HistoryScreen.jsx";
import { SettingsScreen } from "./SettingsScreen.jsx";

let nextId = 100;
const rand = (a, b) => a + Math.random() * (b - a);

const INITIAL_ITEMS = [
  {
    id: 1, title: "conference-talk-1080p.mp4", host: "youtube.com", status: "downloading",
    segments: [100, 100, 84, 61, 90, 42, 100, 77, 55, 68, 100, 31, 88, 49, 72, 60],
    speedMBps: 4.2, etaText: "0:42", downloadedMB: 128.5, totalMB: 704.2,
  },
  {
    id: 2, title: "dataset-archive-2026.zip", host: "archive.org", status: "downloading",
    segments: [100, 62, 38, 55, 20, 74, 41, 12, 66, 30, 48, 25, 58, 15, 33, 44],
    speedMBps: 8.6, etaText: "3:18", downloadedMB: 812.4, totalMB: 1843.0,
  },
  {
    id: 3, title: "field-recording-master.mp3", host: "vimeo.com", status: "paused",
    progress: 44, speedMBps: 0, etaText: "", downloadedMB: 8.1, totalMB: 18.4,
  },
  {
    id: 4, title: "lecture-series-04.mp4", host: "coursera.org", status: "error",
    progress: 54, speedMBps: 0, downloadedMB: 231.0, totalMB: 428.0,
    errorMessage: "Connection reset by server", recoveryHint: "Check your connection and retry — resume picks up from the bytes already on disk.",
  },
  {
    id: 5, title: "podcast-episode-112.mp3", host: "acast.com", status: "queued",
    progress: 0, speedMBps: 0, downloadedMB: 0, totalMB: 96.3,
  },
  {
    id: 6, title: "keynote-recap-720p.mp4", host: "youtube.com", status: "complete",
    progress: 100, speedMBps: 0, downloadedMB: 412.8, totalMB: 412.8,
  },
];

const INITIAL_HISTORY = [
  { id: "h1", title: "keynote-recap-720p.mp4", path: "~/Downloads/Rustloader/HighQuality/2026-07-02/", sizeText: "412.8 MB", dateText: "2026-07-02 18:44", status: "complete" },
  { id: "h2", title: "interview-cut-final.mp4", path: "~/Downloads/Rustloader/HighQuality/2026-06-28/", sizeText: "704.2 MB", dateText: "2026-06-28 09:12", status: "complete" },
  { id: "h3", title: "field-notes-audio.mp3", path: "~/Downloads/Rustloader/Standard/2026-06-15/", sizeText: "Unknown size", dateText: "2026-06-15 12:30", status: "complete" },
  { id: "h4", title: "render-farm-output.mov", path: "~/Downloads/Rustloader/HighQuality/2026-06-11/", sizeText: "2.1 GB", dateText: "2026-06-11 22:03", status: "error" },
];

function tickItem(item) {
  if (item.status !== "downloading") return item;
  if (item.segments) {
    const segments = item.segments.map((s) => (s >= 100 ? 100 : Math.min(100, s + rand(0.4, 2.2))));
    const pct = segments.reduce((a, b) => a + b, 0) / segments.length;
    const downloadedMB = (pct / 100) * item.totalMB;
    const speedMBps = Math.max(0.4, item.speedMBps + rand(-0.5, 0.5));
    const remain = (item.totalMB - downloadedMB) / speedMBps;
    const done = segments.every((s) => s >= 100);
    return done
      ? { ...item, segments: undefined, progress: 100, status: "complete", downloadedMB: item.totalMB, speedMBps: 0, etaText: "" }
      : { ...item, segments, downloadedMB, speedMBps, etaText: `${Math.floor(remain / 60)}:${String(Math.floor(remain % 60)).padStart(2, "0")}` };
  }
  const progress = Math.min(100, (item.progress || 0) + rand(0.5, 1.5));
  return progress >= 100
    ? { ...item, progress: 100, status: "complete", downloadedMB: item.totalMB, speedMBps: 0 }
    : { ...item, progress, downloadedMB: (progress / 100) * item.totalMB, speedMBps: Math.max(0.3, rand(1.5, 3)) };
}

const NAV = [
  { id: "downloads", icon: "download", label: "Downloads" },
  { id: "history", icon: "history", label: "History" },
  { id: "settings", icon: "settings", label: "Settings" },
];

export function AppShell() {
  const [view, setView] = useState("downloads");
  const [light, setLight] = useState(false);
  const [url, setUrl] = useState("");
  const [quality, setQuality] = useState("1080p");
  const [format, setFormat] = useState("MP4");
  const [extracting, setExtracting] = useState(false);
  const [banner, setBanner] = useState("https://vimeo.com/948217345/precision-tooling-demo");
  const [items, setItems] = useState(INITIAL_ITEMS);
  const [records, setRecords] = useState(INITIAL_HISTORY);
  const [settings, setSettings] = useState({
    location: "~/Downloads/Rustloader/", concurrent: 5, segments: 16,
    quality: "Best Available", format: "MP4", cookies: "None", clipboard: false,
  });

  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t = setInterval(() => setItems((its) => its.map(tickItem)), reduced ? 1200 : 400);
    return () => clearInterval(t);
  }, []);

  const queueUrl = (u) => {
    setExtracting(true);
    setTimeout(() => {
      setExtracting(false);
      const name = (u.split("/").pop() || "download").slice(0, 40) || "download";
      let host = "direct";
      try { host = new URL(u).hostname.replace("www.", ""); } catch (e) {}
      setItems((its) => [{
        id: nextId++, title: `${name}.${format.toLowerCase()}`, host, status: "downloading",
        segments: Array.from({ length: settings.segments }, () => rand(0, 8)),
        speedMBps: rand(2, 9), etaText: "", downloadedMB: 0, totalMB: rand(60, 900),
      }, ...its]);
    }, 900);
  };

  const actions = {
    setUrl, setQuality, setFormat,
    pasteUrl: () => setUrl("https://youtube.com/watch?v=dQw4w9WgXcQ"),
    startDownload: () => { if (url) { queueUrl(url); setUrl(""); } },
    confirmBanner: () => { queueUrl(banner); setBanner(null); },
    dismissBanner: () => setBanner(null),
    setStatus: (id, status) => setItems((its) => its.map((i) => (i.id === id ? { ...i, status, speedMBps: status === "downloading" ? Math.max(1, i.speedMBps) : 0 } : i))),
    remove: (id) => setItems((its) => its.filter((i) => i.id !== id)),
    resumeAll: () => setItems((its) => its.map((i) => (i.status === "paused" || i.status === "queued" ? { ...i, status: "downloading" } : i))),
    clearCompleted: () => setItems((its) => its.filter((i) => i.status !== "complete")),
  };

  const active = items.filter((i) => i.status === "downloading");
  const aggregate = active.reduce((a, i) => a + i.speedMBps, 0);
  const viewTitle = NAV.find((n) => n.id === view).label;

  return (
    <div data-theme={light ? "light" : undefined} style={{
      width: "var(--window-w)", height: "var(--window-h)", display: "flex",
      background: "var(--surface-window)", color: "var(--text-body)",
      fontFamily: "var(--font-ui)", overflow: "hidden", borderRadius: 12,
      border: "1px solid var(--border-hairline)", boxSizing: "border-box",
    }}>
      {/* Icon rail */}
      <nav style={{ width: 64, display: "flex", flexDirection: "column", alignItems: "center", gap: 6, padding: "14px 0", background: "var(--surface-panel)", borderRight: "1px solid var(--border-hairline)", boxSizing: "border-box" }}>
        <img src="../../assets/icons/icon_32x32.png" alt="Rustloader" width={30} height={30} style={{ marginBottom: 12 }} />
        {NAV.map((n) => (
          <button
            key={n.id} title={n.label} onClick={() => setView(n.id)}
            style={{
              width: 44, height: 44, display: "flex", alignItems: "center", justifyContent: "center",
              background: view === n.id ? "var(--bg-accent-soft)" : "transparent",
              color: view === n.id ? "var(--rust-300)" : "var(--text-label)",
              border: view === n.id ? "1px solid var(--border-accent)" : "1px solid transparent",
              borderRadius: "var(--radius-md)", cursor: "pointer",
              transition: "background var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out)",
              outline: "none", padding: 0,
            }}
            onFocus={(e) => { e.target.style.boxShadow = "var(--focus-ring)"; }}
            onBlur={(e) => { e.target.style.boxShadow = "none"; }}
          >
            <Icon name={n.icon} size={18} />
          </button>
        ))}
        <span style={{ flex: 1 }}></span>
        <IconButton icon={light ? "clock" : "zap"} title="Toggle light theme" size={36} iconSize={16} onClick={() => setLight(!light)} />
      </nav>

      {/* Main column */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
        {/* Header */}
        <header style={{ display: "flex", alignItems: "center", gap: 14, padding: "14px 24px 12px", borderBottom: "1px solid var(--border-hairline)", flexShrink: 0 }}>
          <div style={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <span style={{ fontSize: "var(--text-2xl)", fontWeight: 600, letterSpacing: "var(--tracking-tight)", lineHeight: 1.15 }}>{viewTitle}</span>
            <span style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-2xs)", letterSpacing: "var(--tracking-caps)", textTransform: "uppercase", color: "var(--text-label)" }}>
              rust<span style={{ color: "var(--rust-400)" }}>loader</span> · v0.9.0
            </span>
          </div>
          <span style={{ flex: 1 }}></span>
          <StatTile label="Active" value={active.length} icon="download" />
          <StatTile label="Aggregate" value={aggregate.toFixed(1)} unit="MB/s" icon="gauge" hot={aggregate > 0} />
        </header>

        {/* View */}
        <main style={{ flex: 1, minHeight: 0, padding: "16px 24px 20px", boxSizing: "border-box" }}>
          {view === "downloads" ? <DownloadsScreen state={{ url, quality, format, extracting, banner, items }} actions={actions} /> : null}
          {view === "history" ? (
            <HistoryScreen
              records={records}
              onRemove={(id) => setRecords((rs) => rs.filter((r) => r.id !== id))}
              onRedownload={() => setView("downloads")}
            />
          ) : null}
          {view === "settings" ? <SettingsScreen settings={settings} setSetting={(k, v) => setSettings((s) => ({ ...s, [k]: v }))} /> : null}
        </main>
      </div>
    </div>
  );
}
