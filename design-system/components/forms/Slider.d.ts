/** Range slider with amber mono value readout — segments (4–32), max concurrent (1–10). */
export interface SliderProps {
  min?: number;
  max?: number;
  value?: number;
  onChange?: (value: number) => void;
  /** Uppercase micro-label; the current value renders right-aligned in amber mono */
  label?: string;
  /** Appended to value readout, e.g. " seg" */
  unit?: string;
  width?: number | string;
  style?: React.CSSProperties;
}
