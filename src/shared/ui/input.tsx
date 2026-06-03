import type { InputHTMLAttributes } from "react";

import { cn } from "@/core/cn";
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
        className={cn("argus-input", error && "border-danger", className)}
        {...props}
      />
      {error && <Text tone="danger" className="text-xs">{error}</Text>}
    </div>
  );
}
