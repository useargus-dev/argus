import { useEffect } from "react";

import { useRegisterStore } from "@/state/register-store";
import { RegisterAccountForm } from "@/features/register/account";
import { RegisterFactorForm } from "@/features/register/factor";
import { RegisterShell } from "./shell";

export function RegisterFlow() {
  const step = useRegisterStore((s) => s.step);

  useEffect(() => {
    return () => useRegisterStore.getState().reset();
  }, []);

  if (step === 1) {
    return (
      <RegisterShell
        step={1}
        title="Setup master account"
        subtitle="Step 1 of 3 — Master account details."
      >
        <RegisterAccountForm />
      </RegisterShell>
    );
  }

  return (
    <RegisterShell
      step={2}
      title="Add a second factor"
      subtitle="Step 2 of 3 — Required. Pick one."
    >
      <RegisterFactorForm />
    </RegisterShell>
  );
}
