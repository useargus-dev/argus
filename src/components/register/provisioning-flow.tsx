import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";

import { ProvisioningSteps } from "../auth/provisioning-steps";
import { AuthLayout } from "../layout/auth-layout";
import { Button } from "../ui/button";
import { useTauriEvent } from "../../hooks/use-tauri-event";
import { bridge } from "../../lib/tauri-bridge";
import { useAuthStore } from "../../state/auth-store";
import type { RegisterProgress } from "../../types/auth";

export function RegisterProvisioningFlow() {
  const navigate = useNavigate();
  const setSignedIn = useAuthStore((s) => s.setSignedIn);
  const [progress, setProgress] = useState<
    Record<string, RegisterProgress["status"]>
  >({});
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [started, setStarted] = useState(false);
  const didAutoRun = useRef(false);

  const runFinalize = useCallback(async () => {
    setErrorMessage(null);
    setStarted(true);
    try {
      await bridge.registerFinalize();
    } catch (e) {
      setErrorMessage(e instanceof Error ? e.message : "Failed to start setup");
      setStarted(false);
    }
  }, []);

  useEffect(() => {
    if (didAutoRun.current) return;
    didAutoRun.current = true;
    void runFinalize();
  }, [runFinalize]);

  useTauriEvent<RegisterProgress>("register-progress", (p) => {
    if (p.step === "error" || p.status === "error") {
      setErrorMessage(p.message ?? "Setup failed");
      setStarted(false);
      return;
    }
    setProgress((prev) => ({ ...prev, [p.step]: p.status }));
    if (p.step === "complete" && p.status === "done") {
      setTimeout(() => navigate("/dashboard", { replace: true }), 400);
    }
  });

  useTauriEvent("signed-in", async () => {
    const scopes = await bridge.getScopeStatus();
    const profile = await bridge.getProfile();
    setSignedIn(profile, scopes);
  });

  return (
    <AuthLayout
      title="Setting up your vault"
      subtitle="Creating encrypted storage and your local account"
    >
      <ProvisioningSteps progress={progress} errorMessage={errorMessage} />
      {errorMessage && (
        <Button
          type="button"
          className="mt-4 w-full"
          onClick={runFinalize}
          disabled={started}
        >
          Retry setup
        </Button>
      )}
    </AuthLayout>
  );
}
