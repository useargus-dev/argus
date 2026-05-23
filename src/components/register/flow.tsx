import { useEffect } from "react";

import { AuthLayout } from "../layout/auth-layout";
import { useRegisterStore } from "../../state/register-store";
import { RegisterAccountForm } from "./account-form";
import { RegisterFactorForm } from "./factor-form";

export function RegisterFlow() {
  const step = useRegisterStore((s) => s.step);

  useEffect(() => {
    return () => useRegisterStore.getState().reset();
  }, []);

  return (
    <AuthLayout
      title="Create your account"
      subtitle={
        step === 1
          ? "Local vault — no cloud account"
          : "Secure your account with a second factor"
      }
    >
      {step === 1 ? <RegisterAccountForm /> : <RegisterFactorForm />}
    </AuthLayout>
  );
}
