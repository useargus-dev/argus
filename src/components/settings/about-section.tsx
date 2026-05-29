import { Info } from "lucide-react";

import { SettingsRow } from "./settings-row";
import { SettingsSection } from "./settings-section";

const VERSION = "0.1.1";
const LICENSE = "Apache-2.0";

export function AboutSection() {
  return (
    <SettingsSection title="About" icon={Info}>
      <SettingsRow label="Version">
        <span className="font-mono text-sm text-text-muted">{VERSION}</span>
      </SettingsRow>
      <SettingsRow label="License">
        <span className="text-sm text-text-muted">{LICENSE}</span>
      </SettingsRow>
    </SettingsSection>
  );
}
