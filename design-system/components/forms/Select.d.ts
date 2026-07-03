/** Pick-list (quality, format, cookies-from-browser) with uppercase mono micro-label. */
export interface SelectProps {
  options?: string[];
  value?: string;
  onChange?: (value: string) => void;
  /** Uppercase micro-label above the field, e.g. "Quality" */
  label?: string;
  width?: number | string;
  disabled?: boolean;
  style?: React.CSSProperties;
}
