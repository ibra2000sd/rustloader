/** Machined panel surface: hairline border + inset top-highlight + soft shadow. Bento building block. */
export interface CardProps {
  /** "card" (default, radius 10) or "panel" (radius 14, panel surface) */
  elevation?: "card" | "panel";
  /** Translucent glass + blur — overlays/banners only; web-only effect */
  glass?: boolean;
  /** Rust hairline border (hero/CTA panels) */
  accent?: boolean;
  /** Default 16 */
  padding?: number | string;
  children?: React.ReactNode;
  style?: React.CSSProperties;
}
