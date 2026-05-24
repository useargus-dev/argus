import type { ReactNode } from "react";

import { cn } from "../../lib/cn";

type BadgeTone = "accent" | "muted" | "danger" | "warning" | "success";

const toneClasses: Record<BadgeTone, string> = {
  accent: "bg-accent/10 text-accent border-accent/30",
  muted: "bg-surface-raised text-text-muted border-border",
  danger: "bg-danger/10 text-danger border-danger/30",
  warning: "bg-warning/10 text-warning border-warning/30",
  success: "bg-success-muted text-success border-success-border",
};

interface SecretBadgeProps {
  children: ReactNode;
  tone?: BadgeTone;
  prefix?: string;
  className?: string;
}

export function SecretBadge({
  children,
  tone = "muted",
  prefix,
  className,
}: SecretBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide",
        toneClasses[tone],
        className,
      )}
    >
      {prefix}
      {children}
    </span>
  );
}
