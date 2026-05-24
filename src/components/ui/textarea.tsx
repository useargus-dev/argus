import type { TextareaHTMLAttributes } from "react";

import { cn } from "../../lib/cn";
import { Label } from "./label";
import { Text } from "./text";

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  error?: string;
}

export function Textarea({ className, label, error, id, ...props }: TextareaProps) {
  const fieldId = id ?? label?.toLowerCase().replace(/\s+/g, "-");
  return (
    <div className="space-y-1.5">
      {label && <Label htmlFor={fieldId}>{label}</Label>}
      <textarea
        id={fieldId}
        className={cn(
          "argus-input min-h-24",
          error && "border-danger",
          className,
        )}
        {...props}
      />
      {error && <Text tone="danger">{error}</Text>}
    </div>
  );
}
