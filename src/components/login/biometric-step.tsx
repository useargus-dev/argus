import { FingerprintPattern } from "lucide-react";

import { BiometricButton } from "../auth/biometric-button";
import { Button } from "../ui/button";

interface BiometricStepProps {
  loading: boolean;
  onSuccess: () => void;
  onBack: () => void;
}

export function LoginBiometricStep({
  loading,
  onSuccess,
  onBack,
}: BiometricStepProps) {
  return (
    <div>
      <div className="mb-4 rounded-md border border-border bg-surface-muted p-4 text-center">
        <FingerprintPattern className="mx-auto size-10 text-primary" aria-hidden />
        <p className="mt-2 text-xs text-text-muted">Verify with your biometric</p>
        <div className="mt-3">
          <BiometricButton onSuccess={onSuccess} />
        </div>
      </div>
      <Button
        type="button"
        variant="ghost"
        className="h-10 w-full"
        onClick={onBack}
        disabled={loading}
      >
        Back
      </Button>
    </div>
  );
}
