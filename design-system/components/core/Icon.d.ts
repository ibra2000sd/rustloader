/**
 * Lucide stroke glyph. Use for all in-app iconography; never emoji or hand-drawn SVG.
 * @startingPoint section="Core" subtitle="Lucide stroke glyphs" viewport="700x140"
 */
export interface IconProps {
  /** Glyph name, e.g. "download", "pause", "play", "x", "check", "folder", "settings", "history", "clipboard", "link", "alert-triangle", "rotate-cw", "trash", "zap", "gauge", "inbox", "clock" */
  name: string;
  /** Pixel size (square). Default 16 */
  size?: number;
  /** Stroke color. Default currentColor */
  color?: string;
  /** Default 1.75 */
  strokeWidth?: number;
  style?: React.CSSProperties;
}
