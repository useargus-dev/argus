import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";

import { Button } from "../ui/button";
import { Spinner } from "../ui/spinner";
import { Text } from "../ui/text";
import { useTauriEvent } from "../../hooks/use-tauri-event";
import { bridge } from "../../lib/tauri-bridge";
import { useAuthStore } from "../../state/auth-store";
import type { RegisterProgress } from "../../types/auth";
import { RegisterCompleteStep } from "./complete-step";
import { RegisterShell } from "./shell";

export function RegisterProvisioningFlow() {
  const navigate = useNavigate();
  const setSignedIn = useAuthStore((s) => s.setSignedIn);
  const [complete, setComplete] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [started, setStarted] = useState(false);
  const didAutoRun = useRef(false);

  const runFinalize = useCallback(async () => {
    setErrorMessage(null);
    setStarted(true);
    setComplete(false);
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
      setComplete(false);
      return;
    }
    if (p.step === "complete" && p.status === "done") {
      setComplete(true);
      setStarted(false);
    }
  });

  useTauriEvent("signed-in", async () => {
    const scopes = await bridge.getScopeStatus();
    const profile = await bridge.getProfile();
    setSignedIn(profile, scopes);
  });

  function handleEnter() {
    navigate("/dashboard", { replace: true });
  }

  if (complete) {
    return (
      <RegisterShell
        step={3}
        title=""
        subtitle=""
      >
        <RegisterCompleteStep onEnter={handleEnter} />
      </RegisterShell>
    );
  }

  return (
    <RegisterShell
      step={3}
      title="Securing your vault"
      subtitle="Step 3 of 3 — Almost done."
    >
      <div className="flex flex-col items-center py-8">
        <Spinner size={32} />
        <Text tone="muted" className="mt-4 text-sm">
          Setting up encrypted storage…
        </Text>
      </div>
      {errorMessage && (
        <>
          <Text tone="danger" className="mb-4 text-center text-sm">
            {errorMessage}
          </Text>
          <Button
            type="button"
            variant="primary"
            className="h-10 w-full"
            onClick={runFinalize}
            disabled={started}
          >
            Retry setup
          </Button>
        </>
      )}
    </RegisterShell>
  );
}
