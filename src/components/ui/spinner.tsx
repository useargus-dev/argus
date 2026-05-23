import { Loader2 } from "lucide-react";

import { cn } from "../../lib/cn";

export function Spinner({ className, size = 18 }: { className?: string; size?: number }) {
  return (
    <Loader2
      className={cn("animate-spin text-primary", className)}
      size={size}
      aria-hidden
    />
  );
}
