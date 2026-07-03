/** Uppercase mono status tag with state dot; active states pulse (reduced-motion safe). */
export interface StatusBadgeProps {
  status?: "queued" | "downloading" | "paused" | "stalled" | "complete" | "error" | "extracting";
  /** Override label, e.g. "Retrying…" */
  label?: string;
  style?: React.CSSProperties;
}
