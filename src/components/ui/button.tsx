import type { ButtonHTMLAttributes } from "react";

import { cn } from "../../lib/cn";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const variants: Record<Variant, string> = {
  primary:
    "bg-primary text-primary-foreground hover:bg-primary-hover disabled:opacity-50",
  secondary:
    "border border-border bg-surface-raised text-text hover:border-text-muted",
  ghost: "text-text hover:bg-surface-raised",
  danger: "bg-danger/15 text-danger hover:bg-danger/25",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
}

export function Button({
  className,
  variant = "primary",
  type = "button",
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        "inline-flex items-center justify-center gap-2 rounded-md px-4 py-2 text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-bg disabled:cursor-not-allowed disabled:pointer-events-none",
        variants[variant],
        className,
      )}
      {...props}
    />
  );
}
