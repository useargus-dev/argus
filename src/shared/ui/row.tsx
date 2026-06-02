import type { HTMLAttributes } from "react";

import { cn } from "@/core/cn";

export function Row({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex gap-2", className)} {...props} />;
}
