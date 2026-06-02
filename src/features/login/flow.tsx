import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "@/core/toast";

import { AuthLayout } from "@/shared/layout/auth-layout";
import { BridgeError, bridge } from "@/core/bridge";
import { useAuthStore } from "@/state/auth-store";
import type { SecondFactorType } from "@/shared/types/auth";
import { LoginBiometricStep } from "@/features/login/bio";
import { LoginPasswordForm } from "@/features/login/pass";
import { LoginTotpForm } from "@/features/login/totp";

type LoginStep = 1 | 2;

export function LoginFlow() {
  const navigate = useNavigate();
  const setSignedIn = useAuthStore((s) => s.setSignedIn);

  const [step, setStep] = useState<LoginStep>(1);
  const [identifier, setIdentifier] = useState("");
  const [password, setPassword] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [secondFactorType, setSecondFactorType] =
    useState<SecondFactorType>("totp");
  const [loading, setLoading] = useState(false);

  async function completeSignIn(opts?: { totpCode?: string; useBiometric?: boolean }) {
    setLoading(true);
    try {
      const profile = await bridge.signIn({
        identifier: identifier.trim(),
        password,
        totpCode: opts?.totpCode,
        useBiometric: opts?.useBiometric,
      });
      const scopes = await bridge.getScopeStatus();
      setSignedIn(profile, scopes);
      navigate("/dashboard", { replace: true });
    } catch (err) {
      if (err instanceof BridgeError && err.code === "SECOND_FACTOR_REQUIRED") {
        const sft = err.secondFactorType ?? "totp";
        setSecondFactorType(sft as SecondFactorType);
        setStep(2);
        toast.info("Second factor required", "Complete verification to continue");
        return;
      }
      toast.fromError(err, "Sign in failed");
    } finally {
      setLoading(false);
    }
  }

  const subtitle =
    step === 1
      ? "Unlock your local Argus vault"
      : secondFactorType === "totp"
        ? "Enter your authenticator code"
        : "Verify with biometric";

  return (
    <AuthLayout title="Sign in" subtitle={subtitle}>
      {step === 1 ? (
        <LoginPasswordForm
          identifier={identifier}
          password={password}
          loading={loading}
          onIdentifierChange={setIdentifier}
          onPasswordChange={setPassword}
          onSubmit={(e) => {
            e.preventDefault();
            void completeSignIn();
          }}
        />
      ) : secondFactorType === "totp" ? (
        <LoginTotpForm
          totpCode={totpCode}
          loading={loading}
          onTotpChange={setTotpCode}
          onBack={() => setStep(1)}
          onSubmit={(e) => {
            e.preventDefault();
            void completeSignIn({ totpCode });
          }}
        />
      ) : (
        <LoginBiometricStep
          loading={loading}
          onSuccess={() => void completeSignIn({ useBiometric: true })}
          onBack={() => setStep(1)}
        />
      )}
    </AuthLayout>
  );
}
