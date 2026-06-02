import { forwardRef, type InputHTMLAttributes } from "react";

import { cn } from "@/core/cn";

export const ArgusInput = forwardRef<
  HTMLInputElement,
  InputHTMLAttributes<HTMLInputElement>
>(function ArgusInput({ className, ...props }, ref) {
  return (
    <input ref={ref} className={cn("argus-input", className)} {...props} />
  );
});
