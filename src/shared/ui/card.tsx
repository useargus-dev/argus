import type { HTMLAttributes } from "react";

import { cn } from "@/core/cn";

export function Card({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("argus-card p-6", className)} {...props} />;
}
