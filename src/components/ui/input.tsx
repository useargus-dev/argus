import type { InputHTMLAttributes } from "react";

import { cn } from "../../lib/cn";
import { Label } from "./label";
import { Text } from "./text";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export function Input({ className, label, error, id, ...props }: InputProps) {
  const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");
  return (
    <div className="space-y-1.5">
      {label && <Label htmlFor={inputId}>{label}</Label>}
      <input
        id={inputId}
        className={cn(
          "w-full rounded-md border border-border bg-surface px-3 py-2 text-sm text-text placeholder:text-text-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent",
          error && "border-danger",
          className,
        )}
        {...props}
      />
      {error && <Text tone="danger" className="text-xs">{error}</Text>}
    </div>
  );
}
