import { invoke } from "@tauri-apps/api/core";

import type {
  AuthErrorPayload,
  RegisterProgress,
  ScopeStatus,
  SecondFactorType,
  TotpSetup,
  UserProfile,
} from "../types/auth";

function parseError(err: unknown): AuthErrorPayload {
  const raw = typeof err === "string" ? err : String(err);
  try {
    return JSON.parse(raw) as AuthErrorPayload;
  } catch {
    return { code: "UNKNOWN", message: raw };
  }
}

export class BridgeError extends Error {
  code: string;
  secondFactorType?: SecondFactorType;

  constructor(payload: AuthErrorPayload) {
    super(payload.message);
    this.code = payload.code;
    this.secondFactorType = payload.secondFactorType;
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw new BridgeError(parseError(e));
  }
}

export const bridge = {
  hasAccount: () => call<boolean>("has_account"),
  prepareTotpSetup: (accountLabel: string) =>
    call<TotpSetup>("prepare_totp_setup", { accountLabel }),
  verifyBiometric: () => call<void>("verify_biometric"),
  registerValidate: (req: {
    email: string;
    username: string;
    password: string;
    secondFactorType: SecondFactorType;
    totpSecret?: string;
    totpCode?: string;
  }) => call<void>("register_validate", { req }),
  registerFinalize: () => call<void>("register_finalize"),
  signIn: (req: {
    identifier: string;
    password: string;
    totpCode?: string;
    useBiometric?: boolean;
  }) => call<UserProfile>("sign_in", { req }),
  signOut: () => call<void>("sign_out"),
  getScopeStatus: () => call<ScopeStatus>("get_scope_status"),
  getProfile: () => call<UserProfile>("get_profile"),
  updateProfile: (avatarUrl?: string) =>
    call<UserProfile>("update_profile", { req: { avatarUrl } }),
  getSecondFactorType: () => call<string>("get_second_factor_type"),
};

export type { RegisterProgress };
