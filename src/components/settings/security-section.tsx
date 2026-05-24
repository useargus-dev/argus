import { useCallback, useEffect, useState } from "react";
import { ShieldCheck } from "lucide-react";

import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import type { SecondFactorStatus } from "../../types/settings";
import { AUTO_LOCK_OPTIONS } from "../../types/settings";
import type { SecondFactorType } from "../../types/auth";
import { SettingsRow } from "./settings-row";
import { SettingsSection } from "./settings-section";
import { SettingsSelect } from "./settings-select";
import { Switch } from "./switch";

interface SecuritySectionProps {
  factorStatus: SecondFactorStatus | null;
  onFactorStatusChange: (s: SecondFactorStatus) => void;
}

export function SecuritySection({
  factorStatus,
  onFactorStatusChange,
}: SecuritySectionProps) {
  const [autoLock, setAutoLock] = useState("30");
  const [lockOnScreenLock, setLockOnScreenLock] = useState(true);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try {
      const s = await bridge.getSettings();
      setAutoLock(s.auto_lock_minutes ?? "30");
      setLockOnScreenLock((s.lock_on_screen_lock ?? "1") === "1");
    } catch {
      toast.error("Could not load security settings");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function persist(key: string, value: string, successMessage: string) {
    try {
      await bridge.setSetting(key, value);
      toast.success(successMessage);
    } catch (e) {
      toast.fromError(e, "Failed to save setting");
      void load();
    }
  }

  async function handleAutoLockChange(value: string) {
    setAutoLock(value);
    await persist("auto_lock_minutes", value, "Auto-lock updated");
  }

  async function handleScreenLockChange(checked: boolean) {
    setLockOnScreenLock(checked);
    await persist(
      "lock_on_screen_lock",
      checked ? "1" : "0",
      checked ? "Screen lock enabled" : "Screen lock disabled",
    );
  }

  async function handleActiveFactorChange(value: string) {
    if (!factorStatus) return;
    try {
      const next = await bridge.setActiveSecondFactor(
        value as SecondFactorType,
      );
      onFactorStatusChange(next);
      toast.success("Second factor updated");
    } catch (e) {
      toast.fromError(e, "Failed to update second factor");
    }
  }

  const secondFactorOptions: { value: string; label: string }[] = [];
  if (factorStatus?.totpEnrolled) {
    secondFactorOptions.push({
      value: "totp",
      label: "TOTP (Authenticator app)",
    });
  }
  if (factorStatus?.biometricEnrolled) {
    secondFactorOptions.push({
      value: "biometric",
      label: "Biometric (Fingerprint / Windows Hello)",
    });
  }

  const canPickFactor = secondFactorOptions.length > 1;

  return (
    <SettingsSection title="Security" icon={ShieldCheck}>
      {loading ? (
        <p className="text-sm text-text-muted">Loading…</p>
      ) : (
        <>
          <SettingsRow label="Auto-lock after">
            <SettingsSelect
              value={autoLock}
              onChange={handleAutoLockChange}
              options={AUTO_LOCK_OPTIONS}
            />
          </SettingsRow>
          <SettingsRow label="Lock on screen lock">
            <Switch
              checked={lockOnScreenLock}
              onChange={handleScreenLockChange}
              aria-label="Lock on screen lock"
            />
          </SettingsRow>
          <SettingsRow
            label="Second factor"
            description={
              !canPickFactor && secondFactorOptions.length === 1
                ? "Register another method below to switch"
                : undefined
            }
          >
            {secondFactorOptions.length === 0 ? (
              <span className="text-sm text-text-muted">None registered</span>
            ) : (
              <SettingsSelect
                value={factorStatus?.activeSecondFactor ?? "totp"}
                onChange={handleActiveFactorChange}
                options={secondFactorOptions}
                disabled={!canPickFactor}
              />
            )}
          </SettingsRow>
        </>
      )}
    </SettingsSection>
  );
}
