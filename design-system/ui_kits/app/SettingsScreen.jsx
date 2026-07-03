// Rustloader Settings view — grounded in settings_view.rs
import React from "react";
import { Card } from "../../components/core/Card.jsx";
import { Button } from "../../components/core/Button.jsx";
import { Input } from "../../components/forms/Input.jsx";
import { Select } from "../../components/forms/Select.jsx";
import { Slider } from "../../components/forms/Slider.jsx";
import { Toggle } from "../../components/forms/Toggle.jsx";

const sectionTitle = { fontFamily: "var(--font-ui)", fontSize: "var(--text-md)", fontWeight: 600, color: "var(--text-body)", margin: 0 };
const help = { fontFamily: "var(--font-ui)", fontSize: "var(--text-sm)", color: "var(--text-muted)", margin: 0, lineHeight: 1.45 };
const microHelp = { fontFamily: "var(--font-ui)", fontSize: "var(--text-xs)", color: "var(--text-label)", margin: 0 };

export function SettingsScreen({ settings, setSetting }) {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12, height: "100%", minHeight: 0 }}>
      <div style={{ flex: 1, overflowY: "auto", minHeight: 0, paddingRight: 2 }}>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12, alignItems: "start" }}>
          <Card padding={20} style={{ display: "flex", flexDirection: "column", gap: 10, gridColumn: "1 / -1" }}>
            <p style={sectionTitle}>Download Location</p>
            <div style={{ display: "flex", gap: 8 }}>
              <Input value={settings.location} onChange={(v) => setSetting("location", v)} mono style={{ flex: 1 }} />
              <Button icon="folder-open">Browse…</Button>
            </div>
            <p style={microHelp}>Files are organized into HighQuality / Standard / LowQuality folders by date.</p>
          </Card>

          <Card padding={20} style={{ display: "flex", flexDirection: "column", gap: 16 }}>
            <p style={sectionTitle}>Performance</p>
            <Slider label="Max concurrent downloads" min={1} max={10} value={settings.concurrent} onChange={(v) => setSetting("concurrent", v)} />
            <Slider label="Segments per download" min={4} max={32} value={settings.segments} onChange={(v) => setSetting("segments", v)} />
            <p style={microHelp}>More segments help on per-connection-throttled links; the engine falls back to a single stream when ranges aren't supported.</p>
          </Card>

          <Card padding={20} style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <p style={sectionTitle}>Defaults</p>
            <div style={{ display: "flex", gap: 12 }}>
              <Select label="Quality" options={["Best Available", "1080p", "720p", "480p"]} value={settings.quality} onChange={(v) => setSetting("quality", v)} width={150} />
              <Select label="Format" options={["MP4", "MP3"]} value={settings.format} onChange={(v) => setSetting("format", v)} width={90} />
            </div>
            <p style={sectionTitle}>YouTube / Authenticated Sites</p>
            <p style={help}>Read cookies from this browser so logged-in / age-restricted videos work. Applies on next launch.</p>
            <Select options={["None", "Chrome", "Firefox", "Safari"]} value={settings.cookies} onChange={(v) => setSetting("cookies", v)} width={180} />
          </Card>

          <Card padding={20} style={{ display: "flex", flexDirection: "column", gap: 10, gridColumn: "1 / -1" }}>
            <p style={sectionTitle}>Clipboard Monitoring</p>
            <p style={help}>
              Watch the clipboard while Rustloader is running. When you copy a web link (http/https), you'll be asked
              whether to download it — nothing downloads automatically, and clipboard contents are never stored or sent anywhere.
            </p>
            <Toggle checked={settings.clipboard} onChange={(v) => setSetting("clipboard", v)} label="Detect copied URLs" />
            <p style={microHelp}>Takes effect immediately; Save Settings keeps it for next launch.</p>
          </Card>
        </div>
      </div>
      <Button variant="primary" icon="check" style={{ alignSelf: "flex-end" }}>Save Settings</Button>
    </div>
  );
}
