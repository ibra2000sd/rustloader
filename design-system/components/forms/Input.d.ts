/** Text field; rust focus ring, red border on error. Use mono for paths/URLs. */
export interface InputProps {
  value?: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  /** Red error border */
  error?: boolean;
  /** Geist Mono — for file paths and URLs */
  mono?: boolean;
  disabled?: boolean;
  style?: React.CSSProperties;
}
