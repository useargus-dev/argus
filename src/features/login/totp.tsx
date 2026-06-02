import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { Field } from "@/shared/ui/field";

interface TotpFormProps {
  totpCode: string;
  loading: boolean;
  onTotpChange: (v: string) => void;
  onBack: () => void;
  onSubmit: (e: React.FormEvent) => void;
}

export function LoginTotpForm({
  totpCode,
  loading,
  onTotpChange,
  onBack,
  onSubmit,
}: TotpFormProps) {
  return (
    <form onSubmit={onSubmit}>
      <Field label="6-digit code">
        <ArgusInput
          inputMode="numeric"
          maxLength={6}
          placeholder="000000"
          value={totpCode}
          onChange={(e) =>
            onTotpChange(e.target.value.replace(/\D/g, "").slice(0, 6))
          }
          autoFocus
          className="text-center font-mono tracking-[0.3em]"
        />
      </Field>
      <div className="mt-5 flex gap-2">
        <Button
          type="button"
          variant="ghost"
          className="h-10 flex-1"
          onClick={onBack}
          disabled={loading}
        >
          Back
        </Button>
        <Button
          type="submit"
          variant="primary"
          className="h-10 flex-1"
          disabled={loading || totpCode.length !== 6}
        >
          {loading ? "Signing in…" : "Sign in"}
        </Button>
      </div>
    </form>
  );
}
