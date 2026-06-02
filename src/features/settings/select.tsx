import { cn } from "@/core/cn";

interface SettingsSelectProps {
  value: string;
  onChange: (value: string) => void;
  options: readonly { value: string; label: string }[];
  disabled?: boolean;
  className?: string;
}

export function SettingsSelect({
  value,
  onChange,
  options,
  disabled,
  className,
}: SettingsSelectProps) {
  return (
    <select
      className={cn("argus-input min-w-[8.5rem]", className)}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
