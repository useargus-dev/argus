import type { FormHTMLAttributes, HTMLAttributes } from "react";

import { cn } from "../../lib/cn";

export function Form({
  className,
  ...props
}: FormHTMLAttributes<HTMLFormElement>) {
  return <form className={cn("space-y-4", className)} {...props} />;
}

export function FormActions({
  className,
  ...props
}: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("flex gap-2", className)} {...props} />;
}
