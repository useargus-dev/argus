import { useState } from "react";
import { Fingerprint, KeyRound } from "lucide-react";
import { QRCodeSVG } from "qrcode.react";
import { toast } from "../../lib/toast";

import { biometryAvailable, isBiometricPlatform } from "../auth/biometric-button";
import { bridge } from "../../lib/tauri-bridge";
import type { SecondFactorStatus } from "../../types/settings";
import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { Spinner } from "../ui/spinner";
import { Text } from "../ui/text";
import { useAuthStore } from "../../state/auth-store";
import { SettingsSection } from "./settings-section";

interface AuthenticationSectionProps {
  factorStatus: SecondFactorStatus | null;
  onFactorStatusChange: (s: SecondFactorStatus) => void;
}

export function AuthenticationSection({
  factorStatus,
  onFactorStatusChange,
}: AuthenticationSectionProps) {
  const profile = useAuthStore((s) => s.profile);
  const [totpOpen, setTotpOpen] = useState(false);
  const [totpSetup, setTotpSetup] = useState<{
    secret: string;
    otpauthUri: string;
  } | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [totpLoading, setTotpLoading] = useState(false);
  const [bioLoading, setBioLoading] = useState(false);

  async function startTotpEnroll() {
    if (!profile?.email) return;
    setTotpOpen(true);
    setTotpCode("");
    setTotpLoading(true);
    try {
      const setup = await bridge.prepareTotpSetup(profile.email);
      setTotpSetup(setup);
    } catch (e) {
      toast.fromError(e, "Failed to start TOTP setup");
      setTotpOpen(false);
    } finally {
      setTotpLoading(false);
    }
  }

  async function confirmTotp() {
    if (!totpSetup || totpCode.length !== 6) return;
    setTotpLoading(true);
    try {
      const next = await bridge.enrollTotp({
        secret: totpSetup.secret,
        totpCode,
      });
      onFactorStatusChange(next);
      setTotpOpen(false);
      setTotpSetup(null);
      toast.success(
        factorStatus?.totpEnrolled ? "Authenticator updated" : "Authenticator registered",
      );
    } catch (e) {
      toast.fromError(e, "Invalid code");
    } finally {
      setTotpLoading(false);
    }
  }

  async function enrollBiometric() {
    if (!isBiometricPlatform()) {
      toast.error("Biometric unlock is not available on this platform.");
      return;
    }
    const available = await biometryAvailable();
    if (!available) {
      toast.error("Biometric authentication is not available on this device.");
      return;
    }
    setBioLoading(true);
    try {
      const next = await bridge.enrollBiometric();
      onFactorStatusChange(next);
      toast.success(
        factorStatus?.biometricEnrolled
          ? "Biometric re-registered"
          : "Biometric registered",
      );
    } catch (e) {
      toast.fromError(e, "Biometric enrollment failed");
    } finally {
      setBioLoading(false);
    }
  }

  return (
    <SettingsSection title="Authentication methods" icon={KeyRound}>
      <div className="space-y-4">
        <div className="rounded-lg border border-border bg-surface-raised p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-sm font-medium text-text">Authenticator app (TOTP)</p>
              <Text tone="muted" className="text-xs">
                {factorStatus?.totpEnrolled ? "Registered" : "Not registered"}
              </Text>
            </div>
            <Button
              type="button"
              variant="secondary"
              className="shrink-0"
              onClick={startTotpEnroll}
            >
              {factorStatus?.totpEnrolled ? "Re-register" : "Set up"}
            </Button>
          </div>
          {totpOpen && (
            <div className="mt-4 border-t border-border pt-4">
              {totpLoading && !totpSetup ? (
                <div className="flex justify-center py-6">
                  <Spinner />
                </div>
              ) : totpSetup ? (
                <>
                  <div className="mx-auto flex size-32 items-center justify-center rounded bg-white p-2">
                    <QRCodeSVG value={totpSetup.otpauthUri} size={112} />
                  </div>
                  <p className="mt-2 text-center text-xs text-text-muted">
                    Scan with your authenticator app, then enter a code
                  </p>
                  <ArgusInput
                    inputMode="numeric"
                    maxLength={6}
                    placeholder="000000"
                    value={totpCode}
                    onChange={(e) =>
                      setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))
                    }
                    className="mt-3 text-center font-mono tracking-[0.3em]"
                  />
                  <div className="mt-3 flex gap-2">
                    <Button
                      type="button"
                      variant="ghost"
                      className="flex-1"
                      onClick={() => {
                        setTotpOpen(false);
                        setTotpSetup(null);
                      }}
                    >
                      Cancel
                    </Button>
                    <Button
                      type="button"
                      variant="primary"
                      className="flex-1"
                      disabled={totpLoading || totpCode.length !== 6}
                      onClick={confirmTotp}
                    >
                      {totpLoading ? "Verifying…" : "Confirm"}
                    </Button>
                  </div>
                </>
              ) : null}
            </div>
          )}
        </div>

        {isBiometricPlatform() && (
          <div className="rounded-lg border border-border bg-surface-raised p-4">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-start gap-3">
                <Fingerprint className="mt-0.5 size-5 text-signal" aria-hidden />
                <div>
                  <p className="text-sm font-medium text-text">
                    Fingerprint / Windows Hello
                  </p>
                  <Text tone="muted" className="text-xs">
                    {factorStatus?.biometricEnrolled
                      ? "Registered"
                      : "Not registered"}
                  </Text>
                </div>
              </div>
              <Button
                type="button"
                variant="secondary"
                className="shrink-0"
                disabled={bioLoading}
                onClick={enrollBiometric}
              >
                {bioLoading
                  ? "Waiting…"
                  : factorStatus?.biometricEnrolled
                    ? "Re-register"
                    : "Set up"}
              </Button>
            </div>
          </div>
        )}
      </div>
    </SettingsSection>
  );
}
