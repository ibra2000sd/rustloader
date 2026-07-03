/** Single-track progress bar; color follows download state (rust/green/amber/red). For segmented downloads prefer SegmentBar. */
export interface ProgressBarProps {
  /** 0–100 */
  value?: number;
  status?: "downloading" | "complete" | "paused" | "stalled" | "error" | "queued";
  /** Track height px. Default 5 */
  height?: number;
  style?: React.CSSProperties;
}
