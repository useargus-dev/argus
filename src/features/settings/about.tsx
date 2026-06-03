import { getVersion } from "@tauri-apps/api/app";
import { Info } from "lucide-react";
import { useEffect, useState } from "react";

import { SettingsRow } from "@/features/settings/row";
import { SettingsSection } from "@/features/settings/section";

const LICENSE = "AGPL-3.0-or-later";

export function AboutSection() {
  const [version, setVersion] = useState("…");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion("dev"));
  }, []);

  return (
    <SettingsSection title="About" icon={Info}>
      <SettingsRow label="Version">
        <span className="font-mono text-sm text-text-muted">{version}</span>
      </SettingsRow>
      <SettingsRow label="License">
        <span className="text-sm text-text-muted">{LICENSE}</span>
      </SettingsRow>
    </SettingsSection>
  );
}
