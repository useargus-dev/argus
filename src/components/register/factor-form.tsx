import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

import {
  biometryAvailable,
  BiometricButton,
  isBiometricPlatform,
} from "../auth/biometric-button";
import { TotpSetupPanel } from "../auth/totp-setup";
import { Button } from "../ui/button";
import { Form, FormActions } from "../ui/form";
import { Row } from "../ui/row";
import { Stack } from "../ui/stack";
import { Text } from "../ui/text";
import { BridgeError, bridge } from "../../lib/tauri-bridge";
import { useRegisterStore } from "../../state/register-store";
import { FactorOption } from "./factor-option";

export function RegisterFactorForm() {
  const navigate = useNavigate();
  const {
    email,
    username,
    password,
    secondFactorType,
    totpSetup,
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

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    try {
      await bridge.registerValidate({
        email: email.trim(),
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

  const accountLabel = email.trim() || username.trim() || "argus@local";

  return (
    <Form onSubmit={handleSubmit}>
      <Stack className="space-y-2">
        <span className="text-sm font-medium text-text">Second factor (required)</span>
        <Row>
          <FactorOption
            active={secondFactorType === "totp"}
            onClick={() => setSecondFactorType("totp")}
            label="Authenticator app"
          />
          {isBiometricPlatform() && bioAvailable && (
            <FactorOption
              active={secondFactorType === "biometric"}
              onClick={() => setSecondFactorType("biometric")}
              label="Biometric"
            />
          )}
        </Row>
        {!isBiometricPlatform() && (
          <Text tone="muted" className="text-xs">
            Linux: biometric unlock is not available — use TOTP.
          </Text>
        )}
      </Stack>

      {secondFactorType === "totp" ? (
        <TotpSetupPanel accountLabel={accountLabel} />
      ) : (
        <BiometricButton
          onSuccess={() => setBiometricReady(true)}
          label={
            biometricReady
              ? "Biometric enrolled ✓"
              : "Enroll fingerprint / Windows Hello"
          }
        />
      )}

      <FormActions>
        <Button
          type="button"
          variant="ghost"
          onClick={() => setStep(1)}
          disabled={loading}
        >
          Back
        </Button>
        <Button
          type="submit"
          className="flex-1"
          disabled={
            loading ||
            (secondFactorType === "totp" && totpCode.length !== 6) ||
            (secondFactorType === "biometric" && !biometricReady)
          }
        >
          {loading ? "Validating…" : "Create account"}
        </Button>
      </FormActions>
    </Form>
  );
}
