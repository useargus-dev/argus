import { create } from "zustand";

import type { SecondFactorType, TotpSetup } from "@/shared/types/auth";

export type RegisterStep = 1 | 2;

interface RegisterState {
  step: RegisterStep;
  firstName: string;
  lastName: string;
  username: string;
  password: string;
  secondFactorType: SecondFactorType;
  totpSetup: TotpSetup | null;
  totpCode: string;
  biometricReady: boolean;
  setStep: (step: RegisterStep) => void;
  setAccount: (
    firstName: string,
    lastName: string,
    username: string,
    password: string,
  ) => void;
  setSecondFactorType: (t: SecondFactorType) => void;
  setTotpSetup: (setup: TotpSetup | null) => void;
  setTotpCode: (code: string) => void;
  setBiometricReady: (v: boolean) => void;
  reset: () => void;
}

const initial = {
  step: 1 as RegisterStep,
  firstName: "",
  lastName: "",
  username: "",
  password: "",
  secondFactorType: "totp" as SecondFactorType,
  totpSetup: null as TotpSetup | null,
  totpCode: "",
  biometricReady: false,
};

export const useRegisterStore = create<RegisterState>((set) => ({
  ...initial,
  setStep: (step) => set({ step }),
  setAccount: (firstName, lastName, username, password) =>
    set({ firstName, lastName, username, password }),
  setSecondFactorType: (secondFactorType) =>
    set({ secondFactorType, biometricReady: false, totpSetup: null, totpCode: "" }),
  setTotpSetup: (totpSetup) => set({ totpSetup }),
  setTotpCode: (totpCode) => set({ totpCode }),
  setBiometricReady: (biometricReady) => set({ biometricReady }),
  reset: () => set({ ...initial }),
}));
