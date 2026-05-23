import { cn } from "../../lib/cn";

interface FactorOptionProps {
  active: boolean;
  onClick: () => void;
  label: string;
}

export function FactorOption({ active, onClick, label }: FactorOptionProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex-1 rounded-md border px-3 py-2 text-sm transition-colors",
        active
          ? "border-accent bg-accent/10 text-text"
          : "border-border text-text-muted hover:border-accent/50",
      )}
    >
      {label}
    </button>
  );
}
