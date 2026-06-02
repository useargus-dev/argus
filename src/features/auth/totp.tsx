import { useEffect, useState } from "react";
import { QRCodeSVG } from "qrcode.react";

import { bridge } from "@/core/bridge";
import { useRegisterStore } from "@/state/register-store";
import { Input } from "@/shared/ui/input";
import { Spinner } from "@/shared/ui/spinner";
import { Text } from "@/shared/ui/text";

export function TotpSetupPanel({ accountLabel }: { accountLabel: string }) {
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
      .catch((e) => setError(e instanceof Error ? e.message : "Failed to load TOTP"))
      .finally(() => setLoading(false));
  }, [accountLabel, totpSetup, setTotpSetup]);

  if (loading) {
    return (
      <div className="flex items-center gap-2">
        <Spinner />
        <Text tone="muted">Generating authenticator secret…</Text>
      </div>
    );
  }

  if (error) {
    return <Text tone="danger">{error}</Text>;
  }

  if (!totpSetup) return null;

  return (
    <div className="space-y-4">
      <Text tone="muted">
        Scan this QR code with your authenticator app, then enter the 6-digit code.
      </Text>
      <div className="flex justify-center rounded-md bg-white p-4">
        <QRCodeSVG value={totpSetup.otpauthUri} size={180} />
      </div>
      <p className="break-all text-center font-mono text-xs text-text-muted">
        {totpSetup.secret}
      </p>
      <Input
        label="Verification code"
        inputMode="numeric"
        maxLength={6}
        placeholder="000000"
        value={totpCode}
        onChange={(e) => setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
      />
    </div>
  );
}
