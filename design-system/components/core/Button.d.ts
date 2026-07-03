/**
 * Button — verb-first labels ("Download", "Pause", "Show in Folder"), never "OK"/"Submit".
 * @startingPoint section="Core" subtitle="Primary / secondary / ghost / destructive" viewport="700x180"
 */
export interface ButtonProps {
  /** Default "secondary". Primary is reserved for the single main action per view. */
  variant?: "primary" | "secondary" | "ghost" | "destructive";
  /** Default "md". "sm" for per-item row controls, "lg" for the hero Download CTA. */
  size?: "sm" | "md" | "lg";
  /** Optional leading Icon name */
  icon?: string;
  disabled?: boolean;
  onClick?: () => void;
  children?: React.ReactNode;
  title?: string;
  style?: React.CSSProperties;
}
