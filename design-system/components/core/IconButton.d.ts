/** Square ghost icon-only control for dense row actions. Always pass `title` (tooltip + aria-label). */
export interface IconButtonProps {
  icon: string;
  /** Square hit target px. Default 28 (min 24; use 32+ for primary surfaces) */
  size?: number;
  iconSize?: number;
  /** Red hover treatment for destructive actions */
  danger?: boolean;
  title?: string;
  onClick?: () => void;
  disabled?: boolean;
  style?: React.CSSProperties;
}
