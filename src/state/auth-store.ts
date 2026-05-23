import { create } from "zustand";

import type { ScopeStatus, UserProfile } from "../types/auth";

interface AuthState {
  isSignedIn: boolean;
  profile: UserProfile | null;
  scopes: ScopeStatus | null;
  setSignedIn: (profile: UserProfile, scopes: ScopeStatus) => void;
  setScopes: (scopes: ScopeStatus) => void;
  clear: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  isSignedIn: false,
  profile: null,
  scopes: null,
  setSignedIn: (profile, scopes) =>
    set({ isSignedIn: true, profile, scopes }),
  setScopes: (scopes) => set({ scopes }),
  clear: () => set({ isSignedIn: false, profile: null, scopes: null }),
}));
