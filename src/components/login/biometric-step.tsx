import { BiometricButton } from "../auth/biometric-button";
import { Button } from "../ui/button";
import { Stack } from "../ui/stack";

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
    <Stack>
      <BiometricButton onSuccess={onSuccess} />
      <Button
        type="button"
        variant="ghost"
        className="w-full"
        onClick={onBack}
        disabled={loading}
      >
        Back
      </Button>
    </Stack>
  );
}
