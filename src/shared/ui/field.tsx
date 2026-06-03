import type { ReactNode } from "react";

import { cn } from "@/core/cn";
import { Text } from "./text";

interface FieldProps {
  label: string;
  children: ReactNode;
  error?: string;
  className?: string;
}

export function Field({ label, children, error, className }: FieldProps) {
  return (
    <label className={cn("block", className)}>
      <span className="mb-1.5 block text-xs font-medium text-text-muted">
        {label}
      </span>
      {children}
      {error && (
        <Text tone="danger" className="mt-1 text-xs">
          {error}
        </Text>
      )}
    </label>
  );
}
