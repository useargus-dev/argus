import { Check } from "lucide-react";

import { Button } from "@/shared/ui/button";

interface CompleteStepProps {
  onEnter: () => void;
}

export function RegisterCompleteStep({ onEnter }: CompleteStepProps) {
  return (
    <div className="py-4 text-center">
      <div className="mx-auto mb-4 grid size-14 place-items-center rounded-full border border-success-border bg-success-muted">
        <Check className="size-7 text-success" aria-hidden />
      </div>
      <h1 className="text-lg font-semibold text-text">Account secured</h1>
      <p className="mb-6 mt-1 text-sm text-text-muted">
        Your vault is ready. Stored locally on this device only.
      </p>
      <Button type="button" variant="primary" className="h-10 w-full" onClick={onEnter}>
        Enter Argus
      </Button>
    </div>
  );
}
