import { useState } from "react";
import { Check, Copy } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Text } from "@/shared/ui/text";
import { toast } from "@/core/toast";
import { formatRecoveryCode } from "@/shared/utils/recovery-code";

interface CompleteStepProps {
  recoveryCode: string;
  onEnter: () => void;
}

export function RegisterCompleteStep({ recoveryCode, onEnter }: CompleteStepProps) {
  const [saved, setSaved] = useState(false);
  const formatted = formatRecoveryCode(recoveryCode);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(formatted);
      toast.success("Recovery code copied");
    } catch {
      toast.error("Could not copy to clipboard");
    }
  }

  return (
    <div className="py-2 text-center">
      <div className="mx-auto mb-4 grid size-14 place-items-center rounded-full border border-success-border bg-success-muted">
        <Check className="size-7 text-success" aria-hidden />
      </div>
      <h1 className="text-lg font-semibold text-text">Account secured</h1>
      <p className="mt-1 text-sm text-text-muted">
        Save your recovery code before continuing. You will need it to reset your
        master password or second factor.
      </p>

      <div className="mt-5 rounded-lg border border-accent/30 bg-accent/10 p-4">
        <Text tone="muted" className="mb-2 text-xs font-medium uppercase tracking-wide">
          Recovery code
        </Text>
        <div className="flex items-center justify-center gap-2">
          <code className="font-mono text-xl tracking-[0.25em] text-text">{formatted}</code>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="shrink-0"
            onClick={handleCopy}
            aria-label="Copy recovery code"
          >
            <Copy size={16} />
          </Button>
        </div>
        <Text tone="muted" className="mt-3 text-xs">
          Store this offline in a safe place. Argus cannot show it again.
        </Text>
      </div>

      <label className="mt-5 flex cursor-pointer items-start gap-2 text-left text-sm text-text-muted">
        <input
          type="checkbox"
          className="mt-0.5"
          checked={saved}
          onChange={(e) => setSaved(e.target.checked)}
        />
        <span>I have saved my recovery code in a safe place</span>
      </label>

      <Button
        type="button"
        variant="primary"
        className="mt-4 h-10 w-full"
        disabled={!saved}
        onClick={onEnter}
      >
        Enter Argus
      </Button>
    </div>
  );
}
