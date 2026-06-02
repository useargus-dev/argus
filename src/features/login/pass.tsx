import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { Field } from "@/shared/ui/field";
import { PasswordInput } from "@/shared/ui/password-input";

interface PasswordFormProps {
  identifier: string;
  password: string;
  loading: boolean;
  onIdentifierChange: (v: string) => void;
  onPasswordChange: (v: string) => void;
  onSubmit: (e: React.FormEvent) => void;
  onForgotPassword?: () => void;
}

export function LoginPasswordForm({
  identifier,
  password,
  loading,
  onIdentifierChange,
  onPasswordChange,
  onSubmit,
  onForgotPassword,
}: PasswordFormProps) {
  return (
    <form onSubmit={onSubmit}>
      <div className="space-y-3">
        <Field label="Email or username">
          <ArgusInput
            autoComplete="username"
            value={identifier}
            onChange={(e) => onIdentifierChange(e.target.value)}
            required
          />
        </Field>
        <PasswordInput
          value={password}
          onChange={onPasswordChange}
          autoComplete="current-password"
        />
      </div>
      {onForgotPassword && (
        <div className="mt-3 text-right">
          <button
            type="button"
            className="text-xs text-accent hover:underline"
            onClick={onForgotPassword}
          >
            Forgot password?
          </button>
        </div>
      )}
      <Button type="submit" variant="primary" className="mt-5 h-10 w-full" disabled={loading}>
        {loading ? "Verifying…" : "Continue"}
      </Button>
    </form>
  );
}
