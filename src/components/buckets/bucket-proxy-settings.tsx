import { useState } from "react";

import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import type { BucketMeta } from "../../types/bucket";
import { AccordionSection } from "../ui/accordion-section";
import { Switch } from "../settings/switch";
import { SecretBadge } from "../secrets/secret-badge";

interface BucketProxySettingsProps {
  bucket: BucketMeta;
  onBucketChange: (bucket: BucketMeta) => void;
}

export function BucketProxySettings({
  bucket,
  onBucketChange,
}: BucketProxySettingsProps) {
  const [toggling, setToggling] = useState(false);

  async function toggleProxy(enabled: boolean) {
    setToggling(true);
    try {
      const updated = await bridge.setBucketProxyEnabled(bucket.id, enabled);
      onBucketChange(updated);
      toast.success(enabled ? "Proxy enabled" : "Proxy disabled");
    } catch (e) {
      toast.fromError(e, "Failed to update proxy");
    } finally {
      setToggling(false);
    }
  }

  return (
    <AccordionSection
      title="Argus Proxy"
      description="Loopback MITM proxy for this bucket. Enable per env key below, then set allowed hosts on each mapping."
      headerAction={
        <Switch
          checked={bucket.proxyEnabled}
          disabled={toggling}
          onChange={toggleProxy}
          aria-label={bucket.proxyEnabled ? "Disable proxy" : "Enable proxy"}
        />
      }
    >
      {bucket.proxyEnabled && bucket.proxyPort != null ? (
        <div className="flex flex-wrap items-center gap-2">
          <SecretBadge tone="accent">127.0.0.1:{bucket.proxyPort}</SecretBadge>
          <span className="text-xs text-text-muted">
            Injected as HTTP_PROXY / HTTPS_PROXY when developers run load_env
          </span>
        </div>
      ) : (
        <p className="text-xs text-text-muted">
          Turn on the switch above to allocate a loopback port for this bucket.
        </p>
      )}
    </AccordionSection>
  );
}
