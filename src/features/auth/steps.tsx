import { Check, Loader2, X } from "lucide-react";

import { cn } from "@/core/cn";
import type { RegisterProgress } from "@/shared/types/auth";
import { PROVISIONING_STEPS } from "@/shared/types/auth";

export function ProvisioningSteps({
  progress,
  errorMessage,
}: {
  progress: Record<string, RegisterProgress["status"]>;
  errorMessage: string | null;
}) {
  return (
    <ul className="space-y-3" aria-live="polite">
      {PROVISIONING_STEPS.map(({ key, label }) => {
        const status = progress[key];
        return (
          <li
            key={key}
            className={cn(
              "flex items-center gap-3 rounded-md border border-border px-3 py-2 text-sm",
              status === "running" && "border-signal bg-surface-muted",
              status === "done" && "text-success",
            )}
          >
            <StepIcon status={status} />
            <span className={status === "done" ? "text-text" : "text-text-muted"}>
              {label}
            </span>
          </li>
        );
      })}
      {errorMessage && (
        <li className="flex items-center gap-3 rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger">
          <X size={18} />
          {errorMessage}
        </li>
      )}
    </ul>
  );
}

function StepIcon({ status }: { status?: RegisterProgress["status"] }) {
  if (status === "done") return <Check size={18} className="text-success" />;
  if (status === "running")
    return <Loader2 size={18} className="animate-spin text-signal" />;
  return <span className="inline-block h-[18px] w-[18px] rounded-full border border-border" />;
}
