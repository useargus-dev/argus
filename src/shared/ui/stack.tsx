import type { HTMLAttributes } from "react";

import { cn } from "@/core/cn";

export function Stack({
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("space-y-4", className)} {...props} />;
}
