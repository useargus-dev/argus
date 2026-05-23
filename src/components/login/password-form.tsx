import { Button } from "../ui/button";
import { Form } from "../ui/form";
import { Input } from "../ui/input";
import { PasswordInput } from "../ui/password-input";

interface PasswordFormProps {
  identifier: string;
  password: string;
  loading: boolean;
  onIdentifierChange: (v: string) => void;
  onPasswordChange: (v: string) => void;
  onSubmit: (e: React.FormEvent) => void;
}

export function LoginPasswordForm({
  identifier,
  password,
  loading,
  onIdentifierChange,
  onPasswordChange,
  onSubmit,
}: PasswordFormProps) {
  return (
    <Form onSubmit={onSubmit}>
      <Input
        label="Email or username"
        autoComplete="username"
        value={identifier}
        onChange={(e) => onIdentifierChange(e.target.value)}
        required
      />
      <PasswordInput
        value={password}
        onChange={onPasswordChange}
        autoComplete="current-password"
      />
      <Button type="submit" className="w-full" disabled={loading}>
        {loading ? "Verifying…" : "Continue"}
      </Button>
    </Form>
  );
}
