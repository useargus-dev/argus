import type { ReactNode } from "react";

import { cn } from "../../lib/cn";

interface FactorCardProps {
  active: boolean;
  onClick: () => void;
  icon: ReactNode;
  title: string;
  description: string;
}

export function FactorCard({
  active,
  onClick,
  icon,
  title,
  description,
}: FactorCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "rounded-lg border p-4 text-left transition-colors",
        active
          ? "border-primary bg-surface-muted"
          : "border-border bg-surface hover:border-text-muted",
      )}
    >
      <div className={cn("mb-2", active ? "text-primary" : "text-text-muted")}>
        {icon}
      </div>
      <div className="text-sm font-medium text-text">{title}</div>
      <div className="text-xs text-text-muted">{description}</div>
    </button>
  );
}
