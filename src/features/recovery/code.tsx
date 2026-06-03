import { useEffect, useState } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { QRCodeSVG } from "qrcode.react";
import { FingerprintPattern, QrCode } from "lucide-react";

import { AuthLayout } from "@/shared/layout/auth-layout";
import { isBiometricPlatform, biometryAvailable } from "@/features/auth/bio";
import { RegisterBiometricPanel } from "@/features/register/bio";
import { FactorCard } from "@/features/register/card";
import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import {
  recoveryCodePath,
  useRecoveryStore,
  type RecoveryIntent,
} from "@/state/recovery-store";
import type { SecondFactorType, TotpSetup } from "@/shared/types/auth";
import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { Spinner } from "@/shared/ui/spinner";
import { Text } from "@/shared/ui/text";
import { RecoveryCodeInput } from "@/shared/ui/recovery-code-input";
import { isValidRecoveryCode } from "@/shared/utils/recovery-code";

function parseIntent(raw: string | null): RecoveryIntent | null {
  if (raw === "password" || raw === "factor") return raw;
  return null;
}

export function RecoveryCodeStep() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const setVerified = useRecoveryStore((s) => s.setVerified);
  const reset = useRecoveryStore((s) => s.reset);

  const fromLock = searchParams.get("from") === "lock";
  const intent = parseIntent(searchParams.get("intent"));

  const [code, setCode] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    reset();
    if (!intent) {
      navigate(fromLock ? recoveryCodePath("factor", true) : recoveryCodePath("password"), {
        replace: true,
      });
    }
  }, [reset, intent, fromLock, navigate]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!intent || !isValidRecoveryCode(code)) {
      toast.error("Enter your full 8-character recovery code");
      return;
    }
    setLoading(true);
    try {
      const result = await bridge.verifyAccountRecovery({ recoveryCode: code });
      setVerified({ ...result, fromLock, intent });
      navigate(intent === "password" ? "/recovery/password" : "/recovery/factor", {
        replace: true,
      });
    } catch (err) {
      toast.fromError(err, "Recovery verification failed");
    } finally {
      setLoading(false);
    }
  }

  if (!intent) return null;

  const title =
    intent === "password" ? "Reset master password" : "Re-register second factor";
  const subtitle =
    intent === "password"
      ? "Enter your recovery code to set a new master password."
      : "Enter your recovery code to register a new second factor.";

  return (
    <AuthLayout title={title} subtitle={subtitle}>
      <form onSubmit={handleSubmit}>
        <RecoveryCodeInput value={code} onChange={setCode} disabled={loading} />
        <Text tone="muted" className="mt-4 text-center text-xs">
          Paste or type your 8-character code (with or without a hyphen).
        </Text>
        <Button
          type="submit"
          variant="primary"
          className="mt-5 h-10 w-full"
          disabled={loading || !isValidRecoveryCode(code)}
        >
          {loading ? "Verifying…" : "Continue"}
        </Button>
      </form>
      <div className="mt-4 text-center">
        <Link
          to={fromLock ? "/dashboard" : "/login"}
          className="text-xs text-accent hover:underline"
        >
          {fromLock ? "Back to unlock" : "Back to sign in"}
        </Link>
      </div>
    </AuthLayout>
  );
}

function RecoveryTotpPanel({
  accountLabel,
  totpSetup,
  totpCode,
  onSetup,
  onCodeChange,
}: {
  accountLabel: string;
  totpSetup: TotpSetup | null;
  totpCode: string;
  onSetup: (setup: TotpSetup) => void;
  onCodeChange: (code: string) => void;
}) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!accountLabel || totpSetup) return;
    setLoading(true);
    setError(null);
    bridge
      .prepareTotpSetup(accountLabel)
      .then(onSetup)
      .catch((e) =>
        setError(e instanceof Error ? e.message : "Failed to load TOTP"),
      )
      .finally(() => setLoading(false));
  }, [accountLabel, totpSetup, onSetup]);

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
          onCodeChange(e.target.value.replace(/\D/g, "").slice(0, 6))
        }
        className="mt-3 bg-surface text-center font-mono tracking-[0.3em]"
      />
    </div>
  );
}

export function RecoveryFactorStep() {
  const navigate = useNavigate();
  const verified = useRecoveryStore((s) => s.verified);
  const intent = useRecoveryStore((s) => s.intent);
  const signedIn = useRecoveryStore((s) => s.signedIn);
  const fromLock = useRecoveryStore((s) => s.fromLock);

  const [secondFactorType, setSecondFactorType] = useState<SecondFactorType>("totp");
  const [totpSetup, setTotpSetup] = useState<TotpSetup | null>(null);
  const [totpCode, setTotpCode] = useState("");
  const [biometricReady, setBiometricReady] = useState(false);
  const [bioAvailable, setBioAvailable] = useState(false);
  const [loading, setLoading] = useState(false);
  const [accountLabel, setAccountLabel] = useState("Argus Master");

  useEffect(() => {
    if (!verified || intent !== "factor") {
      navigate(recoveryCodePath("factor", fromLock), { replace: true });
    }
    // Gate on entry only — do not re-run after success navigation
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    biometryAvailable().then(setBioAvailable);
    if (signedIn) {
      bridge
        .getProfile()
        .then((p) => {
          const name = `${p.firstName} ${p.lastName}`.trim();
          if (name) setAccountLabel(name);
        })
        .catch(() => {});
    }
  }, [signedIn]);

  useEffect(() => {
    if (!bioAvailable && secondFactorType === "biometric") {
      setSecondFactorType("totp");
    }
  }, [bioAvailable, secondFactorType]);

  const showBio = isBiometricPlatform() && bioAvailable;
  const canContinue =
    secondFactorType === "totp" ? totpCode.length === 6 : biometricReady;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    try {
      await bridge.recoveryResetSecondFactor({
        secondFactorType,
        totpSecret: secondFactorType === "totp" ? totpSetup?.secret : undefined,
        totpCode: secondFactorType === "totp" ? totpCode : undefined,
      });
      toast.success("Second factor updated", "Sign in with your new second factor");
      navigate("/login", { replace: true });
    } catch (err) {
      toast.fromError(err, "Second factor reset failed");
    } finally {
      setLoading(false);
    }
  }

  if (!verified || intent !== "factor") return null;

  return (
    <AuthLayout
      title="Re-register second factor"
      subtitle="Choose a new authenticator or biometric. Previous second factors are removed."
    >
      <form onSubmit={handleSubmit}>
        <div className="mb-4 grid grid-cols-2 gap-3">
          <FactorCard
            active={secondFactorType === "totp"}
            onClick={() => {
              setSecondFactorType("totp");
              setBiometricReady(false);
            }}
            icon={<QrCode className="size-5" aria-hidden />}
            title="Authenticator"
            description="TOTP via app"
          />
          {showBio ? (
            <FactorCard
              active={secondFactorType === "biometric"}
              onClick={() => {
                setSecondFactorType("biometric");
                setTotpCode("");
                setTotpSetup(null);
              }}
              icon={<FingerprintPattern className="size-5" aria-hidden />}
              title="Biometric"
              description="Touch ID / Hello"
            />
          ) : (
            <div className="rounded-lg border border-border bg-surface p-4 opacity-50">
              <FingerprintPattern className="mb-2 size-5 text-text-muted" aria-hidden />
              <div className="text-sm font-medium text-text-muted">Biometric</div>
              <div className="text-xs text-text-muted">Unavailable</div>
            </div>
          )}
        </div>

        {!showBio && isBiometricPlatform() === false && (
          <Text tone="muted" className="mb-4 text-xs">
            Linux: biometric unlock is not available — use TOTP.
          </Text>
        )}

        {secondFactorType === "totp" ? (
          <RecoveryTotpPanel
            accountLabel={accountLabel}
            totpSetup={totpSetup}
            totpCode={totpCode}
            onSetup={setTotpSetup}
            onCodeChange={setTotpCode}
          />
        ) : showBio ? (
          <RegisterBiometricPanel
            enrolled={biometricReady}
            onEnrolled={() => setBiometricReady(true)}
          />
        ) : null}

        <Button
          type="submit"
          variant="primary"
          className="mt-5 h-10 w-full"
          disabled={loading || !canContinue}
        >
          {loading ? "Saving…" : "Save second factor"}
        </Button>
      </form>
      <div className="mt-4 text-center">
        <Link
          to={recoveryCodePath("factor", fromLock)}
          className="text-xs text-accent hover:underline"
        >
          Back
        </Link>
      </div>
    </AuthLayout>
  );
}
