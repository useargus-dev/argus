import { useState } from "react";
import { Fingerprint } from "lucide-react";
import { checkStatus } from "@choochmeque/tauri-plugin-biometry-api";

import { bridge } from "@/core/bridge";
import { Button } from "@/shared/ui/button";
import { Text } from "@/shared/ui/text";

interface BiometricButtonProps {
  onSuccess: () => void;
  label?: string;
}

export function BiometricButton({
  onSuccess,
  label = "Use fingerprint / Windows Hello",
}: BiometricButtonProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleClick() {
    setLoading(true);
    setError(null);
    try {
      const status = await checkStatus();
      if (!status.isAvailable) {
        setError("Biometric authentication is not available on this device.");
        return;
      }
      await bridge.verifyBiometric();
      onSuccess();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Biometric error");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="space-y-2">
      <Button
        type="button"
        variant="secondary"
        className="w-full gap-2"
        onClick={handleClick}
        disabled={loading}
      >
        <Fingerprint size={18} />
        {loading ? "Waiting for biometric…" : label}
      </Button>
      {error && <Text tone="danger">{error}</Text>}
    </div>
  );
}

export function isBiometricPlatform(): boolean {
  if (typeof navigator === "undefined") return false;
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("linux")) return false;
  return true;
}

export async function biometryAvailable(): Promise<boolean> {
  if (!isBiometricPlatform()) return false;
  try {
    const status = await checkStatus();
    return status.isAvailable;
  } catch {
    return false;
  }
}
