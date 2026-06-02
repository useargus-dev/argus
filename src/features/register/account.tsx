import { useState } from "react";
import { toast } from "@/core/toast";

import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { Field } from "@/shared/ui/field";
import { PasswordInput } from "@/shared/ui/password-input";
import { useRegisterStore } from "@/state/register-store";

const MIN_PASSWORD = 10;

export function RegisterAccountForm() {
  const { firstName, lastName, username, setAccount, setStep } = useRegisterStore();
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!firstName.trim() || !lastName.trim()) {
      toast.error("First and last name are required");
      return;
    }
    if (username.trim().length < 2) {
      toast.error("Username must be at least 2 characters");
      return;
    }
    if (password.length < MIN_PASSWORD) {
      toast.error(`Master password must be at least ${MIN_PASSWORD} characters`);
      return;
    }
    if (password !== confirm) {
      toast.error("Master passwords do not match");
      return;
    }
    setAccount(firstName.trim(), lastName.trim(), username.trim(), password);
    setStep(2);
  }

  return (
    <form onSubmit={handleSubmit}>
      <div className="space-y-3">
        <div className="grid grid-cols-2 gap-3">
          <Field label="First name">
            <ArgusInput
              autoComplete="given-name"
              value={firstName}
              onChange={(e) =>
                useRegisterStore.setState({ firstName: e.target.value })
              }
              required
            />
          </Field>
          <Field label="Last name">
            <ArgusInput
              autoComplete="family-name"
              value={lastName}
              onChange={(e) =>
                useRegisterStore.setState({ lastName: e.target.value })
              }
              required
            />
          </Field>
        </div>
        <Field label="Username">
          <ArgusInput
            autoComplete="username"
            value={username}
            onChange={(e) =>
              useRegisterStore.setState({ username: e.target.value })
            }
            required
          />
        </Field>
        <PasswordInput
          label="Master password"
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
      <Button type="submit" variant="primary" className="mt-5 h-10 w-full">
        Continue
      </Button>
    </form>
  );
}
