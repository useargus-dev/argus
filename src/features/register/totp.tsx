import { useEffect, useState } from "react";
import { QRCodeSVG } from "qrcode.react";

import { ArgusInput } from "@/shared/ui/argus-input";
import { Spinner } from "@/shared/ui/spinner";
import { Text } from "@/shared/ui/text";
import { bridge } from "@/core/bridge";
import { useRegisterStore } from "@/state/register-store";

export function RegisterTotpPanel({ accountLabel }: { accountLabel: string }) {
  const totpSetup = useRegisterStore((s) => s.totpSetup);
  const totpCode = useRegisterStore((s) => s.totpCode);
  const setTotpSetup = useRegisterStore((s) => s.setTotpSetup);
  const setTotpCode = useRegisterStore((s) => s.setTotpCode);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!accountLabel || totpSetup) return;
    setLoading(true);
    setError(null);
    bridge
      .prepareTotpSetup(accountLabel)
      .then(setTotpSetup)
      .catch((e) =>
        setError(e instanceof Error ? e.message : "Failed to load TOTP"),
      )
      .finally(() => setLoading(false));
  }, [accountLabel, totpSetup, setTotpSetup]);

  if (loading) {
    return (
      <div className="mb-4 flex items-center justify-center gap-2 rounded-md border border-border bg-surface-muted p-8">
        <Spinner />
        <Text tone="muted" className="text-xs">
          Generating QR code…
        </Text>
      </div>
    );
  }

  if (error) {
    return (
      <div className="mb-4 rounded-md border border-danger/40 bg-danger/10 p-4">
        <Text tone="danger" className="text-xs">
          {error}
        </Text>
      </div>
    );
  }

  if (!totpSetup) return null;

  return (
    <div className="mb-4 rounded-md border border-border bg-surface-muted p-4">
      <div className="mx-auto flex size-32 items-center justify-center rounded bg-white p-2">
        <QRCodeSVG value={totpSetup.otpauthUri} size={112} />
      </div>
      <p className="mt-3 text-center text-xs text-text-muted">
        Scan with your authenticator app
      </p>
      <ArgusInput
        placeholder="Enter 6-digit code"
        inputMode="numeric"
        maxLength={6}
        value={totpCode}
        onChange={(e) =>
          setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))
        }
        className="mt-3 bg-surface text-center font-mono tracking-[0.3em]"
      />
    </div>
  );
}
