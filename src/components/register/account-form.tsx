import { useState } from "react";
import { toast } from "../../lib/toast";

import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { Field } from "../ui/field";
import { PasswordInput } from "../ui/password-input";
import { useRegisterStore } from "../../state/register-store";

const MIN_PASSWORD = 10;

export function RegisterAccountForm() {
  const { email, username, firstName, lastName, setAccount, setStep } = useRegisterStore();
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!firstName.trim() || !lastName.trim()) {
      toast.error("First and last name are required");
      return;
    }
    if (password.length < MIN_PASSWORD) {
      toast.error(`Password must be at least ${MIN_PASSWORD} characters`);
      return;
    }
    if (password !== confirm) {
      toast.error("Passwords do not match");
      return;
    }
    setAccount(email.trim(), username.trim(), firstName.trim(), lastName.trim(), password);
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
        <Field label="Email">
          <ArgusInput
            type="email"
            autoComplete="email"
            value={email}
            onChange={(e) =>
              useRegisterStore.setState({ email: e.target.value })
            }
            required
          />
        </Field>
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
          value={password}
          onChange={setPassword}
          autoComplete="new-password"
        />
        <PasswordInput
          label="Confirm password"
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
