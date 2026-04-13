
export interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, disabled }: ToggleProps) {
  return (
    <input
      type="checkbox"
      role="switch"
      className="toggle toggle-primary toggle-sm"
      checked={checked}
      onChange={() => onChange(!checked)}
      disabled={disabled}
    />
  );
}
