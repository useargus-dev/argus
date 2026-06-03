import type { HTMLAttributes } from "react";

import { cn } from "@/core/cn";

type Tone = "default" | "muted" | "danger" | "success";

const tones: Record<Tone, string> = {
  default: "text-text",
  muted: "text-text-muted",
  danger: "text-danger",
  success: "text-success",
};

export function Text({
  tone = "default",
  className,
  ...props
}: HTMLAttributes<HTMLParagraphElement> & { tone?: Tone }) {
  return <p className={cn("text-sm", tones[tone], className)} {...props} />;
}
