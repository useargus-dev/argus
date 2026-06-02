import { useEffect, useState } from "react";
import { Fingerprint, Lock } from "lucide-react";

import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import { useAuthStore } from "@/state/auth-store";
import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { Text } from "@/shared/ui/text";

interface AppLockModalProps {
  open: boolean;
}

export function AppLockModal({ open }: AppLockModalProps) {
  const setScopes = useAuthStore((s) => s.setScopes);
  const [factor, setFactor] = useState<"totp" | "biometric" | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open) return;
    setTotpCode("");
    bridge
      .getSecondFactorType()
      .then((t) => setFactor(t.toLowerCase() as "totp" | "biometric"))
      .catch(() => setFactor("totp"));
  }, [open]);

  async function handleUnlock(totp?: string, biometric?: boolean) {
    setLoading(true);
    try {
      const scopes = await bridge.unlockApp({
        totpCode: totp,
        useBiometric: biometric,
      });
      setScopes(scopes);
    } catch (err) {
      toast.fromError(err, "Unlock failed");
    } finally {
      setLoading(false);
    }
  }

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex flex-col items-center justify-center bg-bg p-6">
      <div className="w-full max-w-md text-center">
        <div className="mx-auto mb-4 grid size-14 place-items-center rounded-full bg-signal/15 text-signal">
          <Lock size={28} aria-hidden />
        </div>
        <h1 className="text-xl font-semibold text-text">Argus is locked</h1>
        <Text tone="muted" className="mt-2 text-sm">
          Verify with your second factor to continue. Your session is still active —
          password is not required until you sign out or restart the app.
        </Text>

        <div className="mt-8">
          {factor === null && <Text tone="muted">Loading…</Text>}

          {factor === "totp" && (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void handleUnlock(totpCode, false);
              }}
            >
              <ArgusInput
                inputMode="numeric"
                maxLength={6}
                placeholder="000000"
                value={totpCode}
                onChange={(e) =>
                  setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))
                }
                autoFocus
                className="text-center font-mono tracking-[0.3em]"
                aria-label="6-digit authenticator code"
              />
              <Button
                type="submit"
                variant="primary"
                className="mt-4 w-full"
                disabled={loading || totpCode.length !== 6}
              >
                {loading ? "Unlocking…" : "Unlock"}
              </Button>
            </form>
          )}

          {factor === "biometric" && (
            <Button
              type="button"
              variant="primary"
              className="w-full gap-2"
              onClick={() => handleUnlock(undefined, true)}
              disabled={loading}
            >
              <Fingerprint size={18} />
              {loading ? "Waiting…" : "Unlock with fingerprint / Windows Hello"}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
