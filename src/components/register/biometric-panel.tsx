import { useState } from "react";
import { FingerprintPattern } from "lucide-react";
import { checkStatus } from "@choochmeque/tauri-plugin-biometry-api";

import { Text } from "../ui/text";
import { bridge } from "../../lib/tauri-bridge";
import { cn } from "../../lib/cn";

interface BiometricPanelProps {
  enrolled: boolean;
  onEnrolled: () => void;
}

export function RegisterBiometricPanel({
  enrolled,
  onEnrolled,
}: BiometricPanelProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleEnroll() {
    if (enrolled || loading) return;
    setLoading(true);
    setError(null);
    try {
      const status = await checkStatus();
      if (!status.isAvailable) {
        setError("Biometric authentication is not available on this device.");
        return;
      }
      await bridge.verifyBiometric();
      onEnrolled();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Biometric error");
    } finally {
      setLoading(false);
    }
  }

  return (
    <button
      type="button"
      onClick={() => void handleEnroll()}
      disabled={loading}
      className={cn(
        "mb-4 w-full rounded-md border border-border bg-surface-muted p-4 text-center transition-colors",
        !enrolled && "hover:border-primary/40",
        enrolled && "border-success-border",
      )}
    >
      <FingerprintPattern
        className={cn(
          "mx-auto size-10",
          enrolled ? "text-success" : "text-primary",
          loading && "animate-pulse",
        )}
        aria-hidden
      />
      <p className="mt-2 text-xs text-text-muted">
        {loading
          ? "Waiting for sensor…"
          : enrolled
            ? "Biometric enrolled"
            : "Touch the sensor to enroll"}
      </p>
      {error && (
        <Text tone="danger" className="mt-2 text-xs">
          {error}
        </Text>
      )}
    </button>
  );
}
