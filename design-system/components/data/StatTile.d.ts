/** Bento stat tile — global header stats (active downloads, aggregate speed). */
export interface StatTileProps {
  /** Uppercase mono micro-label, e.g. "Aggregate" */
  label?: string;
  value?: string | number;
  /** Small muted suffix, e.g. "MB/s" */
  unit?: string;
  /** Optional leading Icon name */
  icon?: string;
  /** Amber "live data" numeral */
  hot?: boolean;
  style?: React.CSSProperties;
}
