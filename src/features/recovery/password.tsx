import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { AuthLayout } from "@/shared/layout/auth-layout";
import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import { recoveryCodePath, useRecoveryStore } from "@/state/recovery-store";
import { Button } from "@/shared/ui/button";
import { PasswordInput } from "@/shared/ui/password-input";

const MIN_PASSWORD = 10;

export function RecoveryPasswordStep() {
  const navigate = useNavigate();
  const verified = useRecoveryStore((s) => s.verified);
  const intent = useRecoveryStore((s) => s.intent);
  const setPrefillUsername = useRecoveryStore((s) => s.setPrefillUsername);

  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!verified || intent !== "password") {
      navigate(recoveryCodePath("password"), { replace: true });
    }
    // Gate on entry only — do not re-run after success navigation
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (password.length < MIN_PASSWORD) {
      toast.error(`Master password must be at least ${MIN_PASSWORD} characters`);
      return;
    }
    if (password !== confirm) {
      toast.error("Master passwords do not match");
      return;
    }
    setLoading(true);
    try {
      const result = await bridge.recoveryResetPassword({ newPassword: password });
      setPrefillUsername(result.username);
      toast.success("Master password updated", "Sign in with your new password");
      navigate("/login", { replace: true });
    } catch (err) {
      toast.fromError(err, "Password reset failed");
    } finally {
      setLoading(false);
    }
  }

  if (!verified || intent !== "password") return null;

  return (
    <AuthLayout
      title="Reset master password"
      subtitle="Choose a new master password for your vault."
    >
      <form onSubmit={handleSubmit}>
        <div className="space-y-3">
          <PasswordInput
            label="New master password"
            value={password}
            onChange={setPassword}
            autoComplete="new-password"
          />
          <PasswordInput
            label="Confirm master password"
            value={confirm}
            onChange={setConfirm}
            autoComplete="new-password"
          />
        </div>
        <Button
          type="submit"
          variant="primary"
          className="mt-5 h-10 w-full"
          disabled={loading}
        >
          {loading ? "Updating…" : "Update password"}
        </Button>
      </form>
      <div className="mt-4 text-center">
        <Link to={recoveryCodePath("password")} className="text-xs text-accent hover:underline">
          Back
        </Link>
      </div>
    </AuthLayout>
  );
}
