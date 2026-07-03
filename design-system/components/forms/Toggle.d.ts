/** Switch for opt-in settings (clipboard monitoring — default off). Rust fill when on. */
export interface ToggleProps {
  checked?: boolean;
  onChange?: (checked: boolean) => void;
  label?: string;
  disabled?: boolean;
  style?: React.CSSProperties;
}
