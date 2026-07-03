/**
 * Signature multi-segment progress: one thin cell per parallel byte-range segment, filling independently.
 * @startingPoint section="Data" subtitle="The signature parallel-segment visual" viewport="700x160"
 */
export interface SegmentBarProps {
  /** Per-segment fill 0–100, one entry per parallel connection (typically 16) */
  segments?: number[];
  /** Cell count when rendering an empty/queued bar without data */
  count?: number;
  status?: "downloading" | "complete" | "paused" | "stalled" | "error" | "queued";
  /** Cell height px. Default 14 */
  height?: number;
  gap?: number;
  style?: React.CSSProperties;
}
