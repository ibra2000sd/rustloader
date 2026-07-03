/* @ds-bundle: {"format":4,"namespace":"RustloaderDesignSystem_48af0e","components":[{"name":"Button","sourcePath":"components/core/Button.jsx"},{"name":"Card","sourcePath":"components/core/Card.jsx"},{"name":"Icon","sourcePath":"components/core/Icon.jsx"},{"name":"ICON_NAMES","sourcePath":"components/core/Icon.jsx"},{"name":"IconButton","sourcePath":"components/core/IconButton.jsx"},{"name":"StatusBadge","sourcePath":"components/core/StatusBadge.jsx"},{"name":"ProgressBar","sourcePath":"components/data/ProgressBar.jsx"},{"name":"SegmentBar","sourcePath":"components/data/SegmentBar.jsx"},{"name":"StatTile","sourcePath":"components/data/StatTile.jsx"},{"name":"ClipboardBanner","sourcePath":"components/downloads/ClipboardBanner.jsx"},{"name":"DownloadItem","sourcePath":"components/downloads/DownloadItem.jsx"},{"name":"HistoryItem","sourcePath":"components/downloads/HistoryItem.jsx"},{"name":"UrlBar","sourcePath":"components/downloads/UrlBar.jsx"},{"name":"Input","sourcePath":"components/forms/Input.jsx"},{"name":"Select","sourcePath":"components/forms/Select.jsx"},{"name":"Slider","sourcePath":"components/forms/Slider.jsx"},{"name":"Toggle","sourcePath":"components/forms/Toggle.jsx"},{"name":"AppShell","sourcePath":"ui_kits/app/AppShell.jsx"},{"name":"DownloadsScreen","sourcePath":"ui_kits/app/DownloadsScreen.jsx"},{"name":"HistoryScreen","sourcePath":"ui_kits/app/HistoryScreen.jsx"},{"name":"SettingsScreen","sourcePath":"ui_kits/app/SettingsScreen.jsx"}],"sourceHashes":{"components/core/Button.jsx":"4679e9ecb9b1","components/core/Card.jsx":"86248e9676a5","components/core/Icon.jsx":"7dd2a8b88c75","components/core/IconButton.jsx":"49e6c9a5404a","components/core/StatusBadge.jsx":"aecc312f33af","components/data/ProgressBar.jsx":"ead8040ceb87","components/data/SegmentBar.jsx":"1c09f965891c","components/data/StatTile.jsx":"a5381d0beece","components/downloads/ClipboardBanner.jsx":"f46d03021311","components/downloads/DownloadItem.jsx":"cb472f8c683b","components/downloads/HistoryItem.jsx":"3231e61f1a6d","components/downloads/UrlBar.jsx":"ad58a259bc86","components/forms/Input.jsx":"e0a18e90e73a","components/forms/Select.jsx":"f9f4ee69e2ef","components/forms/Slider.jsx":"2a9387619e00","components/forms/Toggle.jsx":"2b36e65b011e","ui_kits/app/AppShell.jsx":"052bf412a244","ui_kits/app/DownloadsScreen.jsx":"7992878c7528","ui_kits/app/HistoryScreen.jsx":"244ba18fc2f1","ui_kits/app/SettingsScreen.jsx":"aab53934cadb"},"inlinedExternals":[],"unexposedExports":[]} */

(() => {

const __ds_ns = (window.RustloaderDesignSystem_48af0e = window.RustloaderDesignSystem_48af0e || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/core/Card.jsx
try { (() => {
// Rustloader Card — machined panel: card surface + hairline + inset top-highlight

function Card({
  elevation = "card",
  glass = false,
  accent = false,
  padding = 16,
  children,
  style
}) {
  const bg = glass ? "var(--bg-glass)" : elevation === "panel" ? "var(--surface-panel)" : "var(--surface-card)";
  return /*#__PURE__*/React.createElement("div", {
    style: {
      background: bg,
      backdropFilter: glass ? "var(--glass-blur)" : undefined,
      // web-only; Iced falls back to solid
      border: `1px solid ${accent ? "var(--border-accent)" : "var(--border-hairline)"}`,
      borderRadius: elevation === "panel" ? "var(--radius-xl)" : "var(--radius-lg)",
      boxShadow: glass ? "var(--shadow-3)" : "var(--shadow-1), var(--inset-highlight)",
      padding,
      boxSizing: "border-box",
      ...style
    }
  }, children);
}
Object.assign(__ds_scope, { Card });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Card.jsx", error: String((e && e.message) || e) }); }

// components/core/Icon.jsx
try { (() => {
// Rustloader Icon — Lucide glyphs (ISC), copied path data.
// Stroke-based, 1.75px default, matches the precision-instrument aesthetic.

const PATHS = {
  download: ["M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4", "m7 10 5 5 5-5", "M12 15V3"],
  "arrow-down-to-line": ["M12 17V3", "m6 11 6 6 6-6", "M19 21H5"],
  pause: ["M15 4h3v16h-3z", "M6 4h3v16H6z"],
  play: ["m6 3 14 9-14 9V3z"],
  x: ["M18 6 6 18", "m6 6 12 12"],
  check: ["M20 6 9 17l-5-5"],
  folder: ["M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"],
  "folder-open": ["m6 14 1.45-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.55 6a2 2 0 0 1-1.94 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"],
  settings: ["M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z", "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"],
  history: ["M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8", "M3 3v5h5", "M12 7v5l4 2"],
  clipboard: ["M9 2h6a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1z", "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"],
  link: ["M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71", "M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"],
  "alert-triangle": ["m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z", "M12 9v4", "M12 17h.01"],
  "rotate-cw": ["M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8", "M21 3v5h-5"],
  trash: ["M3 6h18", "M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6", "M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"],
  "chevron-down": ["m6 9 6 6 6-6"],
  zap: ["M13 2 3 14h9l-1 8 10-12h-9l1-8z"],
  clock: ["M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z", "M12 6v6l4 2"],
  gauge: ["m12 14 4-4", "M3.34 19a10 10 0 1 1 17.32 0"],
  inbox: ["M2 12h6l2 3h4l2-3h6", "M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"],
  plus: ["M5 12h14", "M12 5v14"],
  search: ["M11 19a8 8 0 1 0 0-16 8 8 0 0 0 0 16z", "m21 21-4.3-4.3"],
  layers: ["m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z", "m22 17.65-9.17 4.16a2 2 0 0 1-1.66 0L2 17.65", "m22 12.65-9.17 4.16a2 2 0 0 1-1.66 0L2 12.65"]
};
function Icon({
  name,
  size = 16,
  color = "currentColor",
  strokeWidth = 1.75,
  style
}) {
  const paths = PATHS[name] || PATHS["alert-triangle"];
  return /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: color,
    strokeWidth: strokeWidth,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    style: {
      flexShrink: 0,
      ...style
    },
    "aria-hidden": "true"
  }, paths.map((d, i) => /*#__PURE__*/React.createElement("path", {
    key: i,
    d: d
  })));
}
const ICON_NAMES = Object.keys(PATHS);
Object.assign(__ds_scope, { Icon, ICON_NAMES });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Icon.jsx", error: String((e && e.message) || e) }); }

// components/core/Button.jsx
try { (() => {
// Rustloader Button — variants map to the Iced theme's PrimaryButton /
// SecondaryButton / DestructiveButton / IconButton(ghost) styles, restyled
// to the dark rust/copper system.
const {
  useState
} = React;
const SIZES = {
  sm: {
    padding: "5px 10px",
    fontSize: "var(--text-sm)",
    gap: 6,
    iconSize: 13
  },
  md: {
    padding: "8px 16px",
    fontSize: "var(--text-md)",
    gap: 7,
    iconSize: 15
  },
  lg: {
    padding: "11px 22px",
    fontSize: "var(--text-lg)",
    gap: 8,
    iconSize: 16
  }
};
function Button({
  variant = "secondary",
  size = "md",
  icon,
  children,
  disabled,
  onClick,
  style,
  title
}) {
  const [hover, setHover] = useState(false);
  const [active, setActive] = useState(false);
  const s = SIZES[size] || SIZES.md;
  const variants = {
    primary: {
      background: active ? "var(--accent-pressed)" : hover ? "var(--accent-hover)" : "var(--accent)",
      color: "var(--fg-on-accent)",
      border: "1px solid transparent",
      boxShadow: hover && !active ? "var(--shadow-accent)" : "none"
    },
    secondary: {
      background: hover ? "var(--surface-hover)" : "var(--surface-card)",
      color: "var(--text-body)",
      border: "1px solid var(--border-strong)",
      boxShadow: "var(--inset-highlight)"
    },
    ghost: {
      background: hover ? "var(--surface-hover)" : "transparent",
      color: hover ? "var(--text-body)" : "var(--text-muted)",
      border: "1px solid transparent"
    },
    destructive: {
      background: hover ? "var(--bg-danger-soft)" : "transparent",
      color: "var(--danger-400)",
      border: "1px solid transparent"
    }
  };
  const v = variants[variant] || variants.secondary;
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    title: title,
    disabled: disabled,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setActive(false);
    },
    onMouseDown: () => setActive(true),
    onMouseUp: () => setActive(false),
    style: {
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      gap: s.gap,
      padding: s.padding,
      fontSize: s.fontSize,
      fontFamily: "var(--font-ui)",
      fontWeight: 500,
      borderRadius: "var(--radius-md)",
      cursor: disabled ? "default" : "pointer",
      transition: "background var(--dur-fast) var(--ease-out), box-shadow var(--dur-fast) var(--ease-out)",
      outline: "none",
      ...(disabled ? {
        background: "var(--surface-card)",
        color: "var(--fg-disabled)",
        border: "1px solid var(--border-hairline)",
        boxShadow: "none"
      } : v),
      ...style
    },
    onFocus: e => {
      e.target.style.boxShadow = "var(--focus-ring)";
    },
    onBlur: e => {
      e.target.style.boxShadow = v.boxShadow || "none";
    }
  }, icon ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: s.iconSize
  }) : null, children);
}
Object.assign(__ds_scope, { Button });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/Button.jsx", error: String((e && e.message) || e) }); }

// components/core/IconButton.jsx
try { (() => {
// Rustloader IconButton — square ghost control (per-item actions, header)
const {
  useState
} = React;
function IconButton({
  icon,
  size = 28,
  iconSize = 15,
  danger,
  title,
  onClick,
  disabled,
  style
}) {
  const [hover, setHover] = useState(false);
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    title: title,
    "aria-label": title,
    disabled: disabled,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      width: size,
      height: size,
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      background: hover && !disabled ? danger ? "var(--bg-danger-soft)" : "var(--surface-hover)" : "transparent",
      color: disabled ? "var(--fg-disabled)" : danger ? "var(--danger-400)" : hover ? "var(--text-body)" : "var(--text-muted)",
      border: "none",
      borderRadius: "var(--radius-sm)",
      cursor: disabled ? "default" : "pointer",
      transition: "background var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out)",
      outline: "none",
      padding: 0,
      ...style
    },
    onFocus: e => {
      e.target.style.boxShadow = "var(--focus-ring)";
    },
    onBlur: e => {
      e.target.style.boxShadow = "none";
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: iconSize
  }));
}
Object.assign(__ds_scope, { IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/IconButton.jsx", error: String((e && e.message) || e) }); }

// components/core/StatusBadge.jsx
try { (() => {
// Rustloader StatusBadge — download states with dot / pulse

const STATES = {
  queued: {
    color: "var(--text-muted)",
    bg: "transparent",
    label: "Queued"
  },
  downloading: {
    color: "var(--rust-300)",
    bg: "var(--bg-accent-soft)",
    label: "Downloading",
    pulse: true
  },
  paused: {
    color: "var(--warning-400)",
    bg: "var(--bg-warning-soft)",
    label: "Paused"
  },
  stalled: {
    color: "var(--warning-400)",
    bg: "var(--bg-warning-soft)",
    label: "Stalled",
    pulse: true
  },
  complete: {
    color: "var(--success-400)",
    bg: "var(--bg-success-soft)",
    label: "Complete"
  },
  error: {
    color: "var(--danger-400)",
    bg: "var(--bg-danger-soft)",
    label: "Failed"
  },
  extracting: {
    color: "var(--text-muted)",
    bg: "transparent",
    label: "Extracting…",
    pulse: true
  }
};
function StatusBadge({
  status = "queued",
  label,
  style
}) {
  const s = STATES[status] || STATES.queued;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: "inline-flex",
      alignItems: "center",
      gap: 6,
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: s.color,
      background: s.bg,
      border: `1px solid ${s.bg === "transparent" ? "var(--border-hairline)" : "transparent"}`,
      borderRadius: "var(--radius-xs)",
      padding: "3px 8px",
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: 999,
      background: s.color,
      animation: s.pulse ? "rl-pulse 2s var(--ease-in-out) infinite" : "none"
    }
  }), label || s.label);
}
Object.assign(__ds_scope, { StatusBadge });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/core/StatusBadge.jsx", error: String((e && e.message) || e) }); }

// components/data/ProgressBar.jsx
try { (() => {
// Rustloader ProgressBar — single-track progress with state colors

const STATE_COLOR = {
  downloading: "var(--accent)",
  complete: "var(--success-500)",
  paused: "var(--warning-400)",
  stalled: "var(--warning-400)",
  error: "var(--danger-400)",
  queued: "var(--fg-disabled)"
};
function ProgressBar({
  value = 0,
  status = "downloading",
  height = 5,
  style
}) {
  const color = STATE_COLOR[status] || STATE_COLOR.downloading;
  return /*#__PURE__*/React.createElement("div", {
    role: "progressbar",
    "aria-valuenow": Math.round(value),
    "aria-valuemin": 0,
    "aria-valuemax": 100,
    style: {
      height,
      background: "var(--progress-track)",
      borderRadius: "var(--radius-xs)",
      overflow: "hidden",
      width: "100%",
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      height: "100%",
      width: `${Math.max(0, Math.min(100, value))}%`,
      background: color,
      borderRadius: "var(--radius-xs)",
      opacity: status === "paused" || status === "queued" ? 0.55 : 1,
      animation: status === "stalled" ? "rl-pulse 2s var(--ease-in-out) infinite" : "none",
      transition: "width var(--dur-med) var(--ease-out), background var(--dur-med) var(--ease-out)"
    }
  }));
}
Object.assign(__ds_scope, { ProgressBar });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/data/ProgressBar.jsx", error: String((e && e.message) || e) }); }

// components/data/SegmentBar.jsx
try { (() => {
// Rustloader SegmentBar — the signature visual: N parallel byte-range
// segments filling independently, reading as one machined bar.

function SegmentBar({
  segments = [],
  count,
  status = "downloading",
  height = 14,
  gap = 2,
  style
}) {
  // Either pass `segments` (array of 0–100 fills) or `count` for an empty/queued bar
  const cells = segments.length ? segments : Array.from({
    length: count || 16
  }, () => 0);
  const colors = {
    downloading: "var(--accent)",
    complete: "var(--success-500)",
    paused: "var(--warning-400)",
    stalled: "var(--warning-400)",
    error: "var(--danger-400)",
    queued: "var(--fg-disabled)"
  };
  const fill = colors[status] || colors.downloading;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap,
      width: "100%",
      ...style
    },
    "aria-hidden": "true"
  }, cells.map((pct, i) => {
    const p = Math.max(0, Math.min(100, pct));
    const active = status === "downloading" && p > 0 && p < 100;
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        flex: 1,
        height,
        background: "var(--progress-track)",
        borderRadius: 2,
        overflow: "hidden",
        position: "relative"
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        position: "absolute",
        inset: 0,
        width: `${p}%`,
        background: p >= 100 && status === "downloading" ? "var(--rust-600)" : fill,
        opacity: status === "paused" || status === "queued" ? 0.55 : 1,
        animation: status === "stalled" ? "rl-pulse 2s var(--ease-in-out) infinite" : "none",
        transition: "width var(--dur-med) var(--ease-out)",
        boxShadow: active ? "0 0 8px rgba(207,111,56,0.5)" : "none" // web-only glow
      }
    }));
  }));
}
Object.assign(__ds_scope, { SegmentBar });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/data/SegmentBar.jsx", error: String((e && e.message) || e) }); }

// components/data/StatTile.jsx
try { (() => {
// Rustloader StatTile — header global stats (bento tile)

function StatTile({
  label,
  value,
  unit,
  icon,
  hot = false,
  style
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 3,
      background: "var(--surface-card)",
      border: "1px solid var(--border-hairline)",
      borderRadius: "var(--radius-lg)",
      padding: "10px 14px",
      boxShadow: "var(--shadow-1), var(--inset-highlight)",
      minWidth: 110,
      boxSizing: "border-box",
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 5,
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)"
    }
  }, icon ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: 11
  }) : null, label), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-xl)",
      fontWeight: 600,
      color: hot ? "var(--data-hot)" : "var(--text-body)",
      lineHeight: 1.15
    }
  }, value, unit ? /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-xs)",
      fontWeight: 500,
      color: "var(--text-muted)",
      marginLeft: 4
    }
  }, unit) : null));
}
Object.assign(__ds_scope, { StatTile });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/data/StatTile.jsx", error: String((e && e.message) || e) }); }

// components/downloads/ClipboardBanner.jsx
try { (() => {
// Rustloader ClipboardBanner — opt-in clipboard detection prompt.
// Glass overlay card; nothing downloads without the explicit confirm.

function ClipboardBanner({
  url = "",
  onConfirm,
  onDismiss,
  disabled,
  style
}) {
  return /*#__PURE__*/React.createElement(__ds_scope.Card, {
    glass: true,
    accent: true,
    padding: "12px 16px",
    style: {
      display: "flex",
      alignItems: "center",
      gap: 14,
      ...style
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      width: 32,
      height: 32,
      borderRadius: "var(--radius-md)",
      background: "var(--bg-accent-soft)",
      color: "var(--rust-300)",
      flexShrink: 0
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "clipboard",
    size: 16
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 2,
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-md)",
      fontWeight: 500,
      color: "var(--text-body)"
    }
  }, "Copied link detected \u2014 download it?"), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-xs)",
      color: "var(--text-muted)",
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap"
    }
  }, url)), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    variant: "primary",
    size: "sm",
    icon: "download",
    onClick: onConfirm,
    disabled: disabled
  }, "Download"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    variant: "ghost",
    size: "sm",
    onClick: onDismiss
  }, "Dismiss"));
}
Object.assign(__ds_scope, { ClipboardBanner });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/downloads/ClipboardBanner.jsx", error: String((e && e.message) || e) }); }

// components/downloads/DownloadItem.jsx
try { (() => {
// Rustloader DownloadItem — queue row: title, host tag, live data,
// segmented progress, per-state controls. States: queued, downloading,
// paused, stalled, complete, error.

const mono = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  fontWeight: 500
};
function Controls({
  status,
  on = {}
}) {
  switch (status) {
    case "downloading":
      return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        icon: "pause",
        onClick: on.pause
      }, "Pause"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "destructive",
        icon: "x",
        onClick: on.cancel
      }, "Cancel"));
    case "paused":
      return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "primary",
        icon: "play",
        onClick: on.resume
      }, "Resume"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "destructive",
        icon: "x",
        onClick: on.cancel
      }, "Cancel"));
    case "stalled":
      return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "primary",
        icon: "rotate-cw",
        onClick: on.restart
      }, "Restart"), /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "destructive",
        icon: "x",
        onClick: on.cancel
      }, "Cancel"));
    case "complete":
      return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        icon: "folder-open",
        onClick: on.openFolder
      }, "Show in Folder"), /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
        icon: "trash",
        title: "Remove",
        onClick: on.remove
      }));
    case "error":
      return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "primary",
        icon: "rotate-cw",
        onClick: on.retry
      }, "Retry"), /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
        icon: "trash",
        title: "Remove",
        onClick: on.remove
      }));
    default:
      // queued
      return /*#__PURE__*/React.createElement(__ds_scope.Button, {
        size: "sm",
        variant: "destructive",
        icon: "x",
        onClick: on.cancel
      }, "Cancel");
  }
}
function DownloadItem({
  title = "download.mp4",
  host = "",
  status = "queued",
  segments,
  progress = 0,
  speedMBps = 0,
  etaText = "",
  downloadedMB = 0,
  totalMB = 0,
  errorMessage,
  recoveryHint,
  statusLabel,
  on = {},
  style
}) {
  const isSegmented = Array.isArray(segments) && segments.length > 0;
  const pct = isSegmented ? segments.reduce((a, b) => a + b, 0) / segments.length : progress;
  const live = status === "downloading";
  return /*#__PURE__*/React.createElement(__ds_scope.Card, {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 10,
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-lg)",
      fontWeight: 500,
      color: "var(--text-body)",
      flex: 1,
      minWidth: 0,
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap"
    }
  }, title), host ? /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      fontSize: "var(--text-2xs)",
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)",
      border: "1px solid var(--border-hairline)",
      borderRadius: "var(--radius-xs)",
      padding: "3px 7px"
    }
  }, host) : null, /*#__PURE__*/React.createElement(__ds_scope.StatusBadge, {
    status: status,
    label: statusLabel
  })), status === "error" && errorMessage ? /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 4,
      background: "var(--bg-danger-soft)",
      border: "1px solid rgba(217,80,59,0.25)",
      borderRadius: "var(--radius-sm)",
      padding: "8px 10px"
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 6,
      fontSize: "var(--text-sm)",
      fontFamily: "var(--font-ui)",
      color: "var(--danger-400)"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "alert-triangle",
    size: 13
  }), errorMessage), recoveryHint ? /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-xs)",
      fontFamily: "var(--font-ui)",
      color: "var(--text-muted)",
      paddingLeft: 19
    }
  }, recoveryHint) : null) : null, isSegmented && status !== "complete" ? /*#__PURE__*/React.createElement(__ds_scope.SegmentBar, {
    segments: segments,
    status: status
  }) : /*#__PURE__*/React.createElement(__ds_scope.ProgressBar, {
    value: status === "complete" ? 100 : pct,
    status: status
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 14
    }
  }, status === "complete" ? /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      color: "var(--text-muted)"
    }
  }, totalMB ? `${totalMB.toFixed(1)} MB` : "") : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      color: live ? "var(--data-hot)" : "var(--text-muted)"
    }
  }, speedMBps.toFixed(1), " MB/s"), etaText ? /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      color: "var(--text-muted)"
    }
  }, "ETA ", etaText) : null, /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      color: "var(--text-muted)"
    }
  }, totalMB ? `${downloadedMB.toFixed(1)} / ${totalMB.toFixed(1)} MB` : `${downloadedMB.toFixed(1)} MB`), isSegmented ? /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      fontSize: "var(--text-2xs)",
      color: "var(--text-label)"
    }
  }, segments.length, " segments \xB7 ", segments.filter(s => s > 0 && s < 100).length, " active") : null, /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      color: "var(--text-label)"
    }
  }, Math.round(pct), "%")), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(Controls, {
    status: status,
    on: on
  }))));
}
Object.assign(__ds_scope, { DownloadItem });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/downloads/DownloadItem.jsx", error: String((e && e.message) || e) }); }

// components/downloads/HistoryItem.jsx
try { (() => {
// Rustloader HistoryItem — completed-downloads row (read/delete only)

const mono = {
  fontFamily: "var(--font-mono)",
  fontSize: "var(--text-sm)",
  fontWeight: 500,
  color: "var(--text-muted)"
};
function HistoryItem({
  title = "download.mp4",
  path = "",
  sizeText = "Unknown size",
  dateText = "",
  status = "complete",
  on = {},
  style
}) {
  return /*#__PURE__*/React.createElement(__ds_scope.Card, {
    padding: "12px 16px",
    style: {
      display: "flex",
      alignItems: "center",
      gap: 14,
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 3,
      flex: 1,
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-md)",
      fontWeight: 500,
      color: "var(--text-body)",
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap"
    }
  }, title), /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      fontSize: "var(--text-xs)",
      color: "var(--text-label)",
      overflow: "hidden",
      textOverflow: "ellipsis",
      whiteSpace: "nowrap"
    }
  }, path)), /*#__PURE__*/React.createElement("span", {
    style: mono
  }, sizeText), /*#__PURE__*/React.createElement("span", {
    style: {
      ...mono,
      fontSize: "var(--text-xs)"
    }
  }, dateText), /*#__PURE__*/React.createElement(__ds_scope.StatusBadge, {
    status: status
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 6
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    icon: "rotate-cw",
    onClick: on.redownload
  }, "Re-download"), /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
    icon: "folder-open",
    title: "Show in Folder",
    onClick: on.openFolder
  }), /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
    icon: "trash",
    danger: true,
    title: "Remove from history",
    onClick: on.remove
  })));
}
Object.assign(__ds_scope, { HistoryItem });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/downloads/HistoryItem.jsx", error: String((e && e.message) || e) }); }

// components/forms/Input.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// Rustloader Input — text field (URL bar uses UrlBar in downloads/)
const {
  useState
} = React;
function Input({
  value,
  onChange,
  placeholder,
  error,
  mono = false,
  disabled,
  style,
  ...rest
}) {
  const [focus, setFocus] = useState(false);
  return /*#__PURE__*/React.createElement("input", _extends({
    type: "text",
    value: value,
    onChange: e => onChange && onChange(e.target.value),
    placeholder: placeholder,
    disabled: disabled,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      fontFamily: mono ? "var(--font-mono)" : "var(--font-ui)",
      fontSize: "var(--text-md)",
      color: disabled ? "var(--fg-disabled)" : "var(--text-body)",
      background: "var(--bg-input)",
      border: `1px solid ${error ? "var(--danger-400)" : focus ? "var(--border-accent)" : "var(--border-strong)"}`,
      borderRadius: "var(--radius-md)",
      padding: "9px 12px",
      outline: "none",
      boxSizing: "border-box",
      width: "100%",
      boxShadow: focus && !error ? "0 0 0 3px var(--bg-accent-soft)" : "var(--inset-highlight)",
      transition: "border-color var(--dur-fast) var(--ease-out), box-shadow var(--dur-fast) var(--ease-out)",
      ...style
    }
  }, rest));
}
Object.assign(__ds_scope, { Input });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Input.jsx", error: String((e && e.message) || e) }); }

// components/forms/Select.jsx
try { (() => {
// Rustloader Select — quality/format/cookies pick-list
const {
  useState
} = React;
function Select({
  options = [],
  value,
  onChange,
  label,
  width,
  disabled,
  style
}) {
  const [focus, setFocus] = useState(false);
  return /*#__PURE__*/React.createElement("label", {
    style: {
      display: "inline-flex",
      flexDirection: "column",
      gap: 4,
      width,
      ...style
    }
  }, label ? /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)"
    }
  }, label) : null, /*#__PURE__*/React.createElement("span", {
    style: {
      position: "relative",
      display: "inline-flex",
      alignItems: "center"
    }
  }, /*#__PURE__*/React.createElement("select", {
    value: value,
    disabled: disabled,
    onChange: e => onChange && onChange(e.target.value),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      appearance: "none",
      WebkitAppearance: "none",
      width: "100%",
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-sm)",
      fontWeight: 500,
      color: disabled ? "var(--fg-disabled)" : "var(--text-body)",
      background: "var(--bg-input)",
      border: `1px solid ${focus ? "var(--border-accent)" : "var(--border-strong)"}`,
      borderRadius: "var(--radius-md)",
      padding: "7px 30px 7px 10px",
      outline: "none",
      cursor: disabled ? "default" : "pointer",
      boxShadow: focus ? "0 0 0 3px var(--bg-accent-soft)" : "var(--inset-highlight)",
      transition: "border-color var(--dur-fast) var(--ease-out)"
    }
  }, options.map(o => /*#__PURE__*/React.createElement("option", {
    key: o,
    value: o
  }, o))), /*#__PURE__*/React.createElement("span", {
    style: {
      position: "absolute",
      right: 9,
      pointerEvents: "none",
      color: "var(--text-label)",
      display: "flex"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "chevron-down",
    size: 13
  }))));
}
Object.assign(__ds_scope, { Select });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Select.jsx", error: String((e && e.message) || e) }); }

// components/downloads/UrlBar.jsx
try { (() => {
// Rustloader UrlBar — the hero URL input: paste affordance + inline
// quality/format quick-select + primary Download action.
const {
  useState
} = React;
function UrlBar({
  value = "",
  onChange,
  onDownload,
  onPaste,
  quality = "1080p",
  onQualityChange,
  format = "MP4",
  onFormatChange,
  extracting = false,
  error,
  style
}) {
  const [focus, setFocus] = useState(false);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 10,
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 8,
      background: "var(--bg-input)",
      border: `1px solid ${error ? "var(--danger-400)" : focus ? "var(--border-accent)" : "var(--border-strong)"}`,
      borderRadius: "var(--radius-lg)",
      padding: "6px 6px 6px 14px",
      boxShadow: focus && !error ? "0 0 0 3px var(--bg-accent-soft)" : "var(--inset-highlight)",
      transition: "border-color var(--dur-fast) var(--ease-out), box-shadow var(--dur-fast) var(--ease-out)"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "link",
    size: 15,
    color: error ? "var(--danger-400)" : "var(--text-label)"
  }), /*#__PURE__*/React.createElement("input", {
    type: "text",
    value: value,
    onChange: e => onChange && onChange(e.target.value),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    placeholder: "Paste a video or file URL \u2014 1800+ sites supported",
    style: {
      flex: 1,
      background: "transparent",
      border: "none",
      outline: "none",
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-md)",
      color: "var(--text-body)",
      padding: "6px 0",
      minWidth: 0
    }
  }), value ? /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
    icon: "x",
    title: "Clear",
    onClick: () => onChange && onChange("")
  }) : null, /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
    icon: "clipboard",
    title: "Paste from clipboard",
    onClick: onPaste,
    size: 32
  }), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    variant: "primary",
    size: "md",
    icon: extracting ? undefined : "download",
    disabled: !value || extracting,
    onClick: onDownload,
    style: {
      borderRadius: "var(--radius-md)"
    }
  }, extracting ? "Extracting…" : "Download")), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "flex-end",
      gap: 10
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Select, {
    label: "Quality",
    options: ["Best Available", "1080p", "720p", "480p"],
    value: quality,
    onChange: onQualityChange,
    width: 140
  }), /*#__PURE__*/React.createElement(__ds_scope.Select, {
    label: "Format",
    options: ["MP4", "MP3"],
    value: format,
    onChange: onFormatChange,
    width: 90
  }), error ? /*#__PURE__*/React.createElement("span", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 6,
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-sm)",
      color: "var(--danger-400)",
      paddingBottom: 7
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "alert-triangle",
    size: 13
  }), error) : null));
}
Object.assign(__ds_scope, { UrlBar });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/downloads/UrlBar.jsx", error: String((e && e.message) || e) }); }

// components/forms/Slider.jsx
try { (() => {
// Rustloader Slider — segments / concurrency, with live mono value

function Slider({
  min = 0,
  max = 100,
  value = 0,
  onChange,
  label,
  unit = "",
  width,
  style
}) {
  const pct = (value - min) / (max - min) * 100;
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 6,
      width,
      ...style
    }
  }, label !== undefined ? /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      justifyContent: "space-between",
      alignItems: "baseline"
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)"
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-sm)",
      fontWeight: 500,
      color: "var(--data-hot)"
    }
  }, value, unit)) : null, /*#__PURE__*/React.createElement("input", {
    type: "range",
    min: min,
    max: max,
    value: value,
    onChange: e => onChange && onChange(Number(e.target.value)),
    className: "rl-slider",
    style: {
      appearance: "none",
      WebkitAppearance: "none",
      width: "100%",
      height: 4,
      margin: 0,
      borderRadius: "var(--radius-xs)",
      outline: "none",
      cursor: "pointer",
      background: `linear-gradient(to right, var(--accent) ${pct}%, var(--progress-track) ${pct}%)`
    }
  }), /*#__PURE__*/React.createElement("style", null, `
        .rl-slider::-webkit-slider-thumb { -webkit-appearance: none; width: 14px; height: 14px; border-radius: 999px; background: var(--fg-1); border: 2px solid var(--accent); box-shadow: var(--shadow-1); }
        .rl-slider:focus-visible { box-shadow: var(--focus-ring); }
      `));
}
Object.assign(__ds_scope, { Slider });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Slider.jsx", error: String((e && e.message) || e) }); }

// components/forms/Toggle.jsx
try { (() => {
// Rustloader Toggle — opt-in switches (clipboard monitoring), default off

function Toggle({
  checked = false,
  onChange,
  label,
  disabled,
  style
}) {
  return /*#__PURE__*/React.createElement("label", {
    style: {
      display: "inline-flex",
      alignItems: "center",
      gap: 10,
      cursor: disabled ? "default" : "pointer",
      ...style
    }
  }, /*#__PURE__*/React.createElement("button", {
    type: "button",
    role: "switch",
    "aria-checked": checked,
    disabled: disabled,
    onClick: () => onChange && onChange(!checked),
    style: {
      width: 36,
      height: 20,
      borderRadius: 999,
      border: "1px solid var(--border-strong)",
      background: checked ? "var(--accent)" : "var(--progress-track)",
      position: "relative",
      cursor: "inherit",
      padding: 0,
      outline: "none",
      flexShrink: 0,
      transition: "background var(--dur-med) var(--ease-out)",
      opacity: disabled ? 0.5 : 1
    },
    onFocus: e => {
      e.target.style.boxShadow = "var(--focus-ring)";
    },
    onBlur: e => {
      e.target.style.boxShadow = "none";
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: "absolute",
      top: 2,
      left: checked ? 17 : 2,
      width: 14,
      height: 14,
      borderRadius: 999,
      background: checked ? "var(--fg-on-accent)" : "var(--fg-2)",
      transition: "left var(--dur-med) var(--ease-out), background var(--dur-med) var(--ease-out)"
    }
  })), label ? /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-md)",
      color: disabled ? "var(--fg-disabled)" : "var(--text-body)"
    }
  }, label) : null);
}
Object.assign(__ds_scope, { Toggle });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/forms/Toggle.jsx", error: String((e && e.message) || e) }); }

// ui_kits/app/DownloadsScreen.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// Rustloader Downloads (main) view — hero UrlBar, clipboard banner, active queue

function DownloadsScreen({
  state,
  actions
}) {
  const {
    url,
    quality,
    format,
    extracting,
    banner,
    items
  } = state;
  const active = items.filter(i => i.status !== "complete");
  const done = items.filter(i => i.status === "complete");
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 14,
      height: "100%",
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.UrlBar, {
    value: url,
    onChange: actions.setUrl,
    quality: quality,
    onQualityChange: actions.setQuality,
    format: format,
    onFormatChange: actions.setFormat,
    extracting: extracting,
    onDownload: actions.startDownload,
    onPaste: actions.pasteUrl
  }), banner ? /*#__PURE__*/React.createElement(__ds_scope.ClipboardBanner, {
    url: banner,
    onConfirm: actions.confirmBanner,
    onDismiss: actions.dismissBanner,
    disabled: extracting
  }) : null, items.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center",
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: "var(--text-label)",
      display: "flex"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "inbox",
    size: 28,
    strokeWidth: 1.5
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-lg)",
      fontWeight: 500,
      color: "var(--text-muted)"
    }
  }, "No active downloads"), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-md)",
      color: "var(--text-label)"
    }
  }, "Paste a URL above \u2014 or copy one anywhere and Rustloader will offer to queue it.")) : /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)"
    }
  }, "Active queue \xB7 ", active.length), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    icon: "play",
    onClick: actions.resumeAll
  }, "Resume All"), done.length > 0 ? /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    variant: "ghost",
    onClick: actions.clearCompleted
  }, "Clear Completed") : null), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: "auto",
      display: "flex",
      flexDirection: "column",
      gap: 10,
      minHeight: 0,
      paddingRight: 2
    }
  }, items.map(item => /*#__PURE__*/React.createElement(__ds_scope.DownloadItem, _extends({
    key: item.id
  }, item, {
    on: {
      pause: () => actions.setStatus(item.id, "paused"),
      resume: () => actions.setStatus(item.id, "downloading"),
      restart: () => actions.setStatus(item.id, "downloading"),
      retry: () => actions.setStatus(item.id, "downloading"),
      cancel: () => actions.remove(item.id),
      remove: () => actions.remove(item.id),
      openFolder: () => {}
    }
  }))))));
}
Object.assign(__ds_scope, { DownloadsScreen });
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/app/DownloadsScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/app/HistoryScreen.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
// Rustloader History view — persisted downloads, read/delete only

function HistoryScreen({
  records,
  onRemove,
  onRedownload
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 14,
      height: "100%",
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 10
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      fontWeight: 500,
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)"
    }
  }, records.length, " downloads"), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    size: "sm",
    icon: "rotate-cw"
  }, "Refresh")), records.length === 0 ? /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      justifyContent: "center",
      gap: 8
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: "var(--text-label)",
      display: "flex"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "history",
    size: 28,
    strokeWidth: 1.5
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-lg)",
      fontWeight: 500,
      color: "var(--text-muted)"
    }
  }, "No download history yet"), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-ui)",
      fontSize: "var(--text-md)",
      color: "var(--text-label)"
    }
  }, "Downloads you complete will show up here.")) : /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: "auto",
      display: "flex",
      flexDirection: "column",
      gap: 8,
      minHeight: 0,
      paddingRight: 2
    }
  }, records.map(r => /*#__PURE__*/React.createElement(__ds_scope.HistoryItem, _extends({
    key: r.id
  }, r, {
    on: {
      remove: () => onRemove(r.id),
      redownload: () => onRedownload(r),
      openFolder: () => {}
    }
  })))));
}
Object.assign(__ds_scope, { HistoryScreen });
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/app/HistoryScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/app/SettingsScreen.jsx
try { (() => {
// Rustloader Settings view — grounded in settings_view.rs

const sectionTitle = {
  fontFamily: "var(--font-ui)",
  fontSize: "var(--text-md)",
  fontWeight: 600,
  color: "var(--text-body)",
  margin: 0
};
const help = {
  fontFamily: "var(--font-ui)",
  fontSize: "var(--text-sm)",
  color: "var(--text-muted)",
  margin: 0,
  lineHeight: 1.45
};
const microHelp = {
  fontFamily: "var(--font-ui)",
  fontSize: "var(--text-xs)",
  color: "var(--text-label)",
  margin: 0
};
function SettingsScreen({
  settings,
  setSetting
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 12,
      height: "100%",
      minHeight: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      overflowY: "auto",
      minHeight: 0,
      paddingRight: 2
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "grid",
      gridTemplateColumns: "1fr 1fr",
      gap: 12,
      alignItems: "start"
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Card, {
    padding: 20,
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 10,
      gridColumn: "1 / -1"
    }
  }, /*#__PURE__*/React.createElement("p", {
    style: sectionTitle
  }, "Download Location"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 8
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Input, {
    value: settings.location,
    onChange: v => setSetting("location", v),
    mono: true,
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    icon: "folder-open"
  }, "Browse\u2026")), /*#__PURE__*/React.createElement("p", {
    style: microHelp
  }, "Files are organized into HighQuality / Standard / LowQuality folders by date.")), /*#__PURE__*/React.createElement(__ds_scope.Card, {
    padding: 20,
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 16
    }
  }, /*#__PURE__*/React.createElement("p", {
    style: sectionTitle
  }, "Performance"), /*#__PURE__*/React.createElement(__ds_scope.Slider, {
    label: "Max concurrent downloads",
    min: 1,
    max: 10,
    value: settings.concurrent,
    onChange: v => setSetting("concurrent", v)
  }), /*#__PURE__*/React.createElement(__ds_scope.Slider, {
    label: "Segments per download",
    min: 4,
    max: 32,
    value: settings.segments,
    onChange: v => setSetting("segments", v)
  }), /*#__PURE__*/React.createElement("p", {
    style: microHelp
  }, "More segments help on per-connection-throttled links; the engine falls back to a single stream when ranges aren't supported.")), /*#__PURE__*/React.createElement(__ds_scope.Card, {
    padding: 20,
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 12
    }
  }, /*#__PURE__*/React.createElement("p", {
    style: sectionTitle
  }, "Defaults"), /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      gap: 12
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Select, {
    label: "Quality",
    options: ["Best Available", "1080p", "720p", "480p"],
    value: settings.quality,
    onChange: v => setSetting("quality", v),
    width: 150
  }), /*#__PURE__*/React.createElement(__ds_scope.Select, {
    label: "Format",
    options: ["MP4", "MP3"],
    value: settings.format,
    onChange: v => setSetting("format", v),
    width: 90
  })), /*#__PURE__*/React.createElement("p", {
    style: sectionTitle
  }, "YouTube / Authenticated Sites"), /*#__PURE__*/React.createElement("p", {
    style: help
  }, "Read cookies from this browser so logged-in / age-restricted videos work. Applies on next launch."), /*#__PURE__*/React.createElement(__ds_scope.Select, {
    options: ["None", "Chrome", "Firefox", "Safari"],
    value: settings.cookies,
    onChange: v => setSetting("cookies", v),
    width: 180
  })), /*#__PURE__*/React.createElement(__ds_scope.Card, {
    padding: 20,
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 10,
      gridColumn: "1 / -1"
    }
  }, /*#__PURE__*/React.createElement("p", {
    style: sectionTitle
  }, "Clipboard Monitoring"), /*#__PURE__*/React.createElement("p", {
    style: help
  }, "Watch the clipboard while Rustloader is running. When you copy a web link (http/https), you'll be asked whether to download it \u2014 nothing downloads automatically, and clipboard contents are never stored or sent anywhere."), /*#__PURE__*/React.createElement(__ds_scope.Toggle, {
    checked: settings.clipboard,
    onChange: v => setSetting("clipboard", v),
    label: "Detect copied URLs"
  }), /*#__PURE__*/React.createElement("p", {
    style: microHelp
  }, "Takes effect immediately; Save Settings keeps it for next launch.")))), /*#__PURE__*/React.createElement(__ds_scope.Button, {
    variant: "primary",
    icon: "check",
    style: {
      alignSelf: "flex-end"
    }
  }, "Save Settings"));
}
Object.assign(__ds_scope, { SettingsScreen });
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/app/SettingsScreen.jsx", error: String((e && e.message) || e) }); }

// ui_kits/app/AppShell.jsx
try { (() => {
// Rustloader AppShell — fixed 1080×720 window: icon rail, header with
// identity + global stats, view switching, simulated download engine.
const {
  useEffect,
  useState
} = React;
let nextId = 100;
const rand = (a, b) => a + Math.random() * (b - a);
const INITIAL_ITEMS = [{
  id: 1,
  title: "conference-talk-1080p.mp4",
  host: "youtube.com",
  status: "downloading",
  segments: [100, 100, 84, 61, 90, 42, 100, 77, 55, 68, 100, 31, 88, 49, 72, 60],
  speedMBps: 4.2,
  etaText: "0:42",
  downloadedMB: 128.5,
  totalMB: 704.2
}, {
  id: 2,
  title: "dataset-archive-2026.zip",
  host: "archive.org",
  status: "downloading",
  segments: [100, 62, 38, 55, 20, 74, 41, 12, 66, 30, 48, 25, 58, 15, 33, 44],
  speedMBps: 8.6,
  etaText: "3:18",
  downloadedMB: 812.4,
  totalMB: 1843.0
}, {
  id: 3,
  title: "field-recording-master.mp3",
  host: "vimeo.com",
  status: "paused",
  progress: 44,
  speedMBps: 0,
  etaText: "",
  downloadedMB: 8.1,
  totalMB: 18.4
}, {
  id: 4,
  title: "lecture-series-04.mp4",
  host: "coursera.org",
  status: "error",
  progress: 54,
  speedMBps: 0,
  downloadedMB: 231.0,
  totalMB: 428.0,
  errorMessage: "Connection reset by server",
  recoveryHint: "Check your connection and retry — resume picks up from the bytes already on disk."
}, {
  id: 5,
  title: "podcast-episode-112.mp3",
  host: "acast.com",
  status: "queued",
  progress: 0,
  speedMBps: 0,
  downloadedMB: 0,
  totalMB: 96.3
}, {
  id: 6,
  title: "keynote-recap-720p.mp4",
  host: "youtube.com",
  status: "complete",
  progress: 100,
  speedMBps: 0,
  downloadedMB: 412.8,
  totalMB: 412.8
}];
const INITIAL_HISTORY = [{
  id: "h1",
  title: "keynote-recap-720p.mp4",
  path: "~/Downloads/Rustloader/HighQuality/2026-07-02/",
  sizeText: "412.8 MB",
  dateText: "2026-07-02 18:44",
  status: "complete"
}, {
  id: "h2",
  title: "interview-cut-final.mp4",
  path: "~/Downloads/Rustloader/HighQuality/2026-06-28/",
  sizeText: "704.2 MB",
  dateText: "2026-06-28 09:12",
  status: "complete"
}, {
  id: "h3",
  title: "field-notes-audio.mp3",
  path: "~/Downloads/Rustloader/Standard/2026-06-15/",
  sizeText: "Unknown size",
  dateText: "2026-06-15 12:30",
  status: "complete"
}, {
  id: "h4",
  title: "render-farm-output.mov",
  path: "~/Downloads/Rustloader/HighQuality/2026-06-11/",
  sizeText: "2.1 GB",
  dateText: "2026-06-11 22:03",
  status: "error"
}];
function tickItem(item) {
  if (item.status !== "downloading") return item;
  if (item.segments) {
    const segments = item.segments.map(s => s >= 100 ? 100 : Math.min(100, s + rand(0.4, 2.2)));
    const pct = segments.reduce((a, b) => a + b, 0) / segments.length;
    const downloadedMB = pct / 100 * item.totalMB;
    const speedMBps = Math.max(0.4, item.speedMBps + rand(-0.5, 0.5));
    const remain = (item.totalMB - downloadedMB) / speedMBps;
    const done = segments.every(s => s >= 100);
    return done ? {
      ...item,
      segments: undefined,
      progress: 100,
      status: "complete",
      downloadedMB: item.totalMB,
      speedMBps: 0,
      etaText: ""
    } : {
      ...item,
      segments,
      downloadedMB,
      speedMBps,
      etaText: `${Math.floor(remain / 60)}:${String(Math.floor(remain % 60)).padStart(2, "0")}`
    };
  }
  const progress = Math.min(100, (item.progress || 0) + rand(0.5, 1.5));
  return progress >= 100 ? {
    ...item,
    progress: 100,
    status: "complete",
    downloadedMB: item.totalMB,
    speedMBps: 0
  } : {
    ...item,
    progress,
    downloadedMB: progress / 100 * item.totalMB,
    speedMBps: Math.max(0.3, rand(1.5, 3))
  };
}
const NAV = [{
  id: "downloads",
  icon: "download",
  label: "Downloads"
}, {
  id: "history",
  icon: "history",
  label: "History"
}, {
  id: "settings",
  icon: "settings",
  label: "Settings"
}];
function AppShell() {
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
    location: "~/Downloads/Rustloader/",
    concurrent: 5,
    segments: 16,
    quality: "Best Available",
    format: "MP4",
    cookies: "None",
    clipboard: false
  });
  useEffect(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const t = setInterval(() => setItems(its => its.map(tickItem)), reduced ? 1200 : 400);
    return () => clearInterval(t);
  }, []);
  const queueUrl = u => {
    setExtracting(true);
    setTimeout(() => {
      setExtracting(false);
      const name = (u.split("/").pop() || "download").slice(0, 40) || "download";
      let host = "direct";
      try {
        host = new URL(u).hostname.replace("www.", "");
      } catch (e) {}
      setItems(its => [{
        id: nextId++,
        title: `${name}.${format.toLowerCase()}`,
        host,
        status: "downloading",
        segments: Array.from({
          length: settings.segments
        }, () => rand(0, 8)),
        speedMBps: rand(2, 9),
        etaText: "",
        downloadedMB: 0,
        totalMB: rand(60, 900)
      }, ...its]);
    }, 900);
  };
  const actions = {
    setUrl,
    setQuality,
    setFormat,
    pasteUrl: () => setUrl("https://youtube.com/watch?v=dQw4w9WgXcQ"),
    startDownload: () => {
      if (url) {
        queueUrl(url);
        setUrl("");
      }
    },
    confirmBanner: () => {
      queueUrl(banner);
      setBanner(null);
    },
    dismissBanner: () => setBanner(null),
    setStatus: (id, status) => setItems(its => its.map(i => i.id === id ? {
      ...i,
      status,
      speedMBps: status === "downloading" ? Math.max(1, i.speedMBps) : 0
    } : i)),
    remove: id => setItems(its => its.filter(i => i.id !== id)),
    resumeAll: () => setItems(its => its.map(i => i.status === "paused" || i.status === "queued" ? {
      ...i,
      status: "downloading"
    } : i)),
    clearCompleted: () => setItems(its => its.filter(i => i.status !== "complete"))
  };
  const active = items.filter(i => i.status === "downloading");
  const aggregate = active.reduce((a, i) => a + i.speedMBps, 0);
  const viewTitle = NAV.find(n => n.id === view).label;
  return /*#__PURE__*/React.createElement("div", {
    "data-theme": light ? "light" : undefined,
    style: {
      width: "var(--window-w)",
      height: "var(--window-h)",
      display: "flex",
      background: "var(--surface-window)",
      color: "var(--text-body)",
      fontFamily: "var(--font-ui)",
      overflow: "hidden",
      borderRadius: 12,
      border: "1px solid var(--border-hairline)",
      boxSizing: "border-box"
    }
  }, /*#__PURE__*/React.createElement("nav", {
    style: {
      width: 64,
      display: "flex",
      flexDirection: "column",
      alignItems: "center",
      gap: 6,
      padding: "14px 0",
      background: "var(--surface-panel)",
      borderRight: "1px solid var(--border-hairline)",
      boxSizing: "border-box"
    }
  }, /*#__PURE__*/React.createElement("img", {
    src: "../../assets/icons/icon_32x32.png",
    alt: "Rustloader",
    width: 30,
    height: 30,
    style: {
      marginBottom: 12
    }
  }), NAV.map(n => /*#__PURE__*/React.createElement("button", {
    key: n.id,
    title: n.label,
    onClick: () => setView(n.id),
    style: {
      width: 44,
      height: 44,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      background: view === n.id ? "var(--bg-accent-soft)" : "transparent",
      color: view === n.id ? "var(--rust-300)" : "var(--text-label)",
      border: view === n.id ? "1px solid var(--border-accent)" : "1px solid transparent",
      borderRadius: "var(--radius-md)",
      cursor: "pointer",
      transition: "background var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out)",
      outline: "none",
      padding: 0
    },
    onFocus: e => {
      e.target.style.boxShadow = "var(--focus-ring)";
    },
    onBlur: e => {
      e.target.style.boxShadow = "none";
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: n.icon,
    size: 18
  }))), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.IconButton, {
    icon: light ? "clock" : "zap",
    title: "Toggle light theme",
    size: 36,
    iconSize: 16,
    onClick: () => setLight(!light)
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      display: "flex",
      flexDirection: "column",
      minWidth: 0
    }
  }, /*#__PURE__*/React.createElement("header", {
    style: {
      display: "flex",
      alignItems: "center",
      gap: 14,
      padding: "14px 24px 12px",
      borderBottom: "1px solid var(--border-hairline)",
      flexShrink: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: "flex",
      flexDirection: "column",
      gap: 1
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: "var(--text-2xl)",
      fontWeight: 600,
      letterSpacing: "var(--tracking-tight)",
      lineHeight: 1.15
    }
  }, viewTitle), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: "var(--font-mono)",
      fontSize: "var(--text-2xs)",
      letterSpacing: "var(--tracking-caps)",
      textTransform: "uppercase",
      color: "var(--text-label)"
    }
  }, "rust", /*#__PURE__*/React.createElement("span", {
    style: {
      color: "var(--rust-400)"
    }
  }, "loader"), " \xB7 v0.9.0")), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), /*#__PURE__*/React.createElement(__ds_scope.StatTile, {
    label: "Active",
    value: active.length,
    icon: "download"
  }), /*#__PURE__*/React.createElement(__ds_scope.StatTile, {
    label: "Aggregate",
    value: aggregate.toFixed(1),
    unit: "MB/s",
    icon: "gauge",
    hot: aggregate > 0
  })), /*#__PURE__*/React.createElement("main", {
    style: {
      flex: 1,
      minHeight: 0,
      padding: "16px 24px 20px",
      boxSizing: "border-box"
    }
  }, view === "downloads" ? /*#__PURE__*/React.createElement(__ds_scope.DownloadsScreen, {
    state: {
      url,
      quality,
      format,
      extracting,
      banner,
      items
    },
    actions: actions
  }) : null, view === "history" ? /*#__PURE__*/React.createElement(__ds_scope.HistoryScreen, {
    records: records,
    onRemove: id => setRecords(rs => rs.filter(r => r.id !== id)),
    onRedownload: () => setView("downloads")
  }) : null, view === "settings" ? /*#__PURE__*/React.createElement(__ds_scope.SettingsScreen, {
    settings: settings,
    setSetting: (k, v) => setSettings(s => ({
      ...s,
      [k]: v
    }))
  }) : null)));
}
Object.assign(__ds_scope, { AppShell });
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/app/AppShell.jsx", error: String((e && e.message) || e) }); }

__ds_ns.Button = __ds_scope.Button;

__ds_ns.Card = __ds_scope.Card;

__ds_ns.Icon = __ds_scope.Icon;

__ds_ns.ICON_NAMES = __ds_scope.ICON_NAMES;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.StatusBadge = __ds_scope.StatusBadge;

__ds_ns.ProgressBar = __ds_scope.ProgressBar;

__ds_ns.SegmentBar = __ds_scope.SegmentBar;

__ds_ns.StatTile = __ds_scope.StatTile;

__ds_ns.ClipboardBanner = __ds_scope.ClipboardBanner;

__ds_ns.DownloadItem = __ds_scope.DownloadItem;

__ds_ns.HistoryItem = __ds_scope.HistoryItem;

__ds_ns.UrlBar = __ds_scope.UrlBar;

__ds_ns.Input = __ds_scope.Input;

__ds_ns.Select = __ds_scope.Select;

__ds_ns.Slider = __ds_scope.Slider;

__ds_ns.Toggle = __ds_scope.Toggle;

__ds_ns.AppShell = __ds_scope.AppShell;

__ds_ns.DownloadsScreen = __ds_scope.DownloadsScreen;

__ds_ns.HistoryScreen = __ds_scope.HistoryScreen;

__ds_ns.SettingsScreen = __ds_scope.SettingsScreen;

})();
