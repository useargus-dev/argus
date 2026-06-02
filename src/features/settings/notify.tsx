import { useCallback, useEffect, useState } from "react";
import { Bell } from "lucide-react";

import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import { EXPIRY_NOTIFY_OPTIONS } from "@/shared/types/settings";
import { SettingsRow } from "@/features/settings/row";
import { SettingsSection } from "@/features/settings/section";
import { SettingsSelect } from "@/features/settings/select";
import { Switch } from "@/shared/ui/switch";

export function NotificationsSection() {
  const [notifyClient, setNotifyClient] = useState(true);
  const [expiryDays, setExpiryDays] = useState("7");
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try {
      const s = await bridge.getSettings();
      setNotifyClient((s.notify_client_access ?? "1") === "1");
      setExpiryDays(s.expiry_notify_days ?? "7");
    } catch {
      toast.error("Could not load notification settings");
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
      toast.fromError(e, "Failed to save");
      void load();
    }
  }

  return (
    <SettingsSection title="Notifications" icon={Bell}>
      {loading ? (
        <p className="text-sm text-text-muted">Loading…</p>
      ) : (
        <>
          <SettingsRow label="Notify on new client access">
            <Switch
              checked={notifyClient}
              onChange={(v) => {
                setNotifyClient(v);
                void persist(
                  "notify_client_access",
                  v ? "1" : "0",
                  v ? "Client access notifications on" : "Client access notifications off",
                );
              }}
              aria-label="Notify on new client access"
            />
          </SettingsRow>
          <SettingsRow label="Notify when secrets expire within">
            <SettingsSelect
              value={expiryDays}
              onChange={(v) => {
                setExpiryDays(v);
                void persist("expiry_notify_days", v, "Expiry reminder updated");
              }}
              options={EXPIRY_NOTIFY_OPTIONS}
            />
          </SettingsRow>
        </>
      )}
    </SettingsSection>
  );
}
