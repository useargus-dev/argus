import type { HTMLAttributes, ReactNode } from "react";
import { AlertCircle, CheckCircle2, Info } from "lucide-react";

import { cn } from "../../lib/cn";

type Variant = "info" | "error" | "success";

const styles: Record<Variant, string> = {
  info: "border-border bg-surface-raised text-text-muted",
  error: "border-danger/50 bg-danger/10 text-danger",
  success: "border-success/50 bg-success/10 text-success",
};

const icons: Record<Variant, ReactNode> = {
  info: <Info size={18} />,
  error: <AlertCircle size={18} />,
  success: <CheckCircle2 size={18} />,
};

export function Banner({
  variant = "info",
  children,
  className,
  ...props
}: HTMLAttributes<HTMLDivElement> & { variant?: Variant }) {
  return (
    <div
      role="alert"
      className={cn(
        "flex items-start gap-3 rounded-md border px-3 py-2 text-sm",
        styles[variant],
        className,
      )}
      {...props}
    >
      {icons[variant]}
      <div className="flex-1">{children}</div>
    </div>
  );
}
