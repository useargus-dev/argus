import { create } from "zustand";

export type RecoveryIntent = "password" | "factor";

interface RecoveryState {
  verified: boolean;
  signedIn: boolean;
  appLocked: boolean;
  fromLock: boolean;
  intent: RecoveryIntent | null;
  prefillUsername: string | null;
  setVerified: (payload: {
    signedIn: boolean;
    appLocked: boolean;
    fromLock?: boolean;
    intent: RecoveryIntent;
  }) => void;
  setPrefillUsername: (username: string) => void;
  takePrefillUsername: () => string | null;
  reset: () => void;
}

const initial = {
  verified: false,
  signedIn: false,
  appLocked: false,
  fromLock: false,
  intent: null as RecoveryIntent | null,
  prefillUsername: null as string | null,
};

export const useRecoveryStore = create<RecoveryState>((set, get) => ({
  ...initial,
  setVerified: ({ signedIn, appLocked, fromLock = false, intent }) =>
    set({ verified: true, signedIn, appLocked, fromLock, intent }),
  setPrefillUsername: (username) => set({ prefillUsername: username }),
  takePrefillUsername: () => {
    const { prefillUsername } = get();
    if (!prefillUsername) return null;
    set({ prefillUsername: null });
    return prefillUsername;
  },
  reset: () => set({ ...initial }),
}));

export function recoveryCodePath(intent: RecoveryIntent, fromLock = false): string {
  const params = new URLSearchParams({ intent });
  if (fromLock) params.set("from", "lock");
  return `/recovery?${params.toString()}`;
}
