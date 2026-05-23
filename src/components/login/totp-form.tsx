import { Button } from "../ui/button";
import { Form, FormActions } from "../ui/form";
import { Input } from "../ui/input";

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
    <Form onSubmit={onSubmit}>
      <Input
        label="6-digit code"
        inputMode="numeric"
        maxLength={6}
        placeholder="000000"
        value={totpCode}
        onChange={(e) => onTotpChange(e.target.value.replace(/\D/g, "").slice(0, 6))}
        autoFocus
      />
      <FormActions>
        <Button type="button" variant="ghost" onClick={onBack} disabled={loading}>
          Back
        </Button>
        <Button
          type="submit"
          className="flex-1"
          disabled={loading || totpCode.length !== 6}
        >
          {loading ? "Signing in…" : "Sign in"}
        </Button>
      </FormActions>
    </Form>
  );
}
