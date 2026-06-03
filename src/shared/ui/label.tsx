import type { LabelHTMLAttributes } from "react";

import { cn } from "@/core/cn";

export function Label({
  className,
  ...props
}: LabelHTMLAttributes<HTMLLabelElement>) {
  return (
    // eslint-disable-next-line jsx-a11y/label-has-associated-control -- primitive; callers pass htmlFor
    <label
      className={cn("text-sm font-medium text-text", className)}
      {...props}
    />
  );
}
