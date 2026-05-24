import { useCallback, useEffect, useState } from "react";
import { Package } from "lucide-react";

import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import { SettingsRow } from "./settings-row";
import { SettingsSection } from "./settings-section";
import { Switch } from "./switch";

export function BackgroundSection() {
  const [runInBackground, setRunInBackground] = useState(true);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try {
      const s = await bridge.getSettings();
      setRunInBackground((s.run_in_background ?? "1") === "1");
    } catch {
      toast.error("Could not load background settings");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function handleChange(checked: boolean) {
    setRunInBackground(checked);
    try {
      await bridge.setSetting("run_in_background", checked ? "1" : "0");
      toast.success(
        checked ? "Background mode enabled" : "Background mode disabled",
      );
    } catch (e) {
      toast.fromError(e, "Failed to save");
      void load();
    }
  }

  return (
    <SettingsSection title="Background" icon={Package}>
      {loading ? (
        <p className="text-sm text-text-muted">Loading…</p>
      ) : (
        <SettingsRow label="Run in system tray when window closed">
          <Switch
            checked={runInBackground}
            onChange={handleChange}
            aria-label="Run in system tray when window closed"
          />
        </SettingsRow>
      )}
    </SettingsSection>
  );
}
