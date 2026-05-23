import { useState } from "react";
import { toast } from "sonner";

import { Button } from "../ui/button";
import { Form } from "../ui/form";
import { Input } from "../ui/input";
import { PasswordInput } from "../ui/password-input";
import { useRegisterStore } from "../../state/register-store";

const MIN_PASSWORD = 10;

export function RegisterAccountForm() {
  const { email, username, setAccount, setStep } = useRegisterStore();
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (password.length < MIN_PASSWORD) {
      toast.error(`Password must be at least ${MIN_PASSWORD} characters`);
      return;
    }
    if (password !== confirm) {
      toast.error("Passwords do not match");
      return;
    }
    setAccount(email.trim(), username.trim(), password);
    setStep(2);
  }

  return (
    <Form onSubmit={handleSubmit}>
      <Input
        label="Email"
        type="email"
        autoComplete="email"
        value={email}
        onChange={(e) => useRegisterStore.setState({ email: e.target.value })}
        required
      />
      <Input
        label="Username"
        autoComplete="username"
        value={username}
        onChange={(e) => useRegisterStore.setState({ username: e.target.value })}
        required
      />
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
      <Button type="submit" className="w-full">
        Continue
      </Button>
    </Form>
  );
}
