import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { FingerprintPattern, QrCode } from "lucide-react";
import { toast } from "@/core/toast";

import { isBiometricPlatform, biometryAvailable } from "@/features/auth/bio";
import { Button } from "@/shared/ui/button";
import { Text } from "@/shared/ui/text";
import { BridgeError, bridge } from "@/core/bridge";
import { useRegisterStore } from "@/state/register-store";
import { FactorCard } from "@/features/register/card";
import { RegisterBiometricPanel } from "@/features/register/bio";
import { RegisterTotpPanel } from "@/features/register/totp";

export function RegisterFactorForm() {
  const navigate = useNavigate();
  const {
    firstName,
    lastName,
    username,
    password,
    secondFactorType,
    totpCode,
    biometricReady,
    setStep,
    setSecondFactorType,
    setBiometricReady,
  } = useRegisterStore();

  const [loading, setLoading] = useState(false);
  const [bioAvailable, setBioAvailable] = useState(false);

  useEffect(() => {
    biometryAvailable().then(setBioAvailable);
  }, []);

  useEffect(() => {
    if (!bioAvailable && secondFactorType === "biometric") {
      setSecondFactorType("totp");
    }
  }, [bioAvailable, secondFactorType, setSecondFactorType]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    try {
      const totpSetup = useRegisterStore.getState().totpSetup;
      await bridge.registerValidate({
        firstName: firstName.trim(),
        lastName: lastName.trim(),
        username: username.trim(),
        password,
        secondFactorType,
        totpSecret: secondFactorType === "totp" ? totpSetup?.secret : undefined,
        totpCode: secondFactorType === "totp" ? totpCode : undefined,
      });
      navigate("/register/provisioning", { replace: true });
    } catch (err) {
      const msg =
        err instanceof BridgeError ? err.message : "Registration validation failed";
      toast.error(msg);
    } finally {
      setLoading(false);
    }
  }

  const accountLabel =
    `${firstName.trim()} ${lastName.trim()}`.trim() || "Argus Master";
  const showBio = isBiometricPlatform() && bioAvailable;

  const canContinue =
    secondFactorType === "totp"
      ? totpCode.length === 6
      : biometricReady;

  return (
    <form onSubmit={handleSubmit}>
      <div className="mb-4 grid grid-cols-2 gap-3">
        <FactorCard
          active={secondFactorType === "totp"}
          onClick={() => setSecondFactorType("totp")}
          icon={<QrCode className="size-5" aria-hidden />}
          title="Authenticator"
          description="TOTP via app"
        />
        {showBio ? (
          <FactorCard
            active={secondFactorType === "biometric"}
            onClick={() => setSecondFactorType("biometric")}
            icon={<FingerprintPattern className="size-5" aria-hidden />}
            title="Biometric"
            description="Touch ID / Hello"
          />
        ) : (
          <div className="rounded-lg border border-border bg-surface p-4 opacity-50">
            <FingerprintPattern className="mb-2 size-5 text-text-muted" aria-hidden />
            <div className="text-sm font-medium text-text-muted">Biometric</div>
            <div className="text-xs text-text-muted">Unavailable</div>
          </div>
        )}
      </div>

      {!showBio && isBiometricPlatform() === false && (
        <Text tone="muted" className="mb-4 text-xs">
          Linux: biometric unlock is not available — use TOTP.
        </Text>
      )}

      {secondFactorType === "totp" ? (
        <RegisterTotpPanel accountLabel={accountLabel} />
      ) : showBio ? (
        <RegisterBiometricPanel
          enrolled={biometricReady}
          onEnrolled={() => setBiometricReady(true)}
        />
      ) : null}

      <div className="flex gap-2">
        <Button
          type="button"
          variant="ghost"
          className="h-10 flex-1"
          onClick={() => setStep(1)}
          disabled={loading}
        >
          Back
        </Button>
        <Button
          type="submit"
          variant="primary"
          className="h-10 flex-1"
          disabled={loading || !canContinue}
        >
          {loading ? "Validating…" : "Continue"}
        </Button>
      </div>
    </form>
  );
}
