import { invoke } from "@tauri-apps/api/core";

import type {
  AuthErrorPayload,
  RegisterProgress,
  ScopeStatus,
  SecondFactorType,
  TotpSetup,
  UserProfile,
} from "@/shared/types/auth";
import type {
  BucketMapping,
  BucketMeta,
  BucketWithToken,
  CreateBucketInput,
  UpsertMappingInput,
} from "@/shared/types/bucket";
import type { ClientAccessRequest, GrantRow } from "@/shared/types/client";
import type { SecretDetail, SecretMeta, SecretWriteInput } from "@/shared/types/secret";
import type { SecondFactorStatus } from "@/shared/types/settings";

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
    firstName: string;
    lastName: string;
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
  unlockApp: (req: { totpCode?: string; useBiometric?: boolean }) =>
    call<ScopeStatus>("unlock_app", { req }),
  lockApp: () => call<ScopeStatus>("lock_app"),
  getScopeStatus: () => call<ScopeStatus>("get_scope_status"),
  getProfile: () => call<UserProfile>("get_profile"),
  updateProfile: (req: { email?: string; username?: string; firstName?: string; lastName?: string }) =>
    call<UserProfile>("update_profile", { req }),
  getSecondFactorStatus: () =>
    call<SecondFactorStatus>("get_second_factor_status"),
  enrollTotp: (req: { secret: string; totpCode: string }) =>
    call<SecondFactorStatus>("enroll_totp", { req }),
  enrollBiometric: () => call<SecondFactorStatus>("enroll_biometric"),
  setActiveSecondFactor: (secondFactorType: SecondFactorType) =>
    call<SecondFactorStatus>("set_active_second_factor", {
      req: { secondFactorType },
    }),
  getSecondFactorType: () => call<string>("get_second_factor_type"),
  elevateVault: (req: { totpCode?: string; useBiometric?: boolean }) =>
    call<ScopeStatus>("elevate_vault", { req }),
  lockVault: () => call<ScopeStatus>("lock_vault"),
  searchSecrets: (query?: string) =>
    call<SecretMeta[]>("search_secrets", { query: query ?? null }),
  getSecret: (id: string) => call<SecretDetail>("get_secret", { id }),
  createSecret: (req: SecretWriteInput) =>
    call<SecretMeta>("create_secret", { req }),
  updateSecret: (id: string, req: SecretWriteInput) =>
    call<SecretMeta>("update_secret", { id, req }),
  deleteSecret: (id: string) => call<void>("delete_secret", { id }),
  listBuckets: () => call<BucketMeta[]>("list_buckets"),
  createBucket: (req: CreateBucketInput) =>
    call<BucketWithToken>("create_bucket", { req }),
  deleteBucket: (id: string) => call<void>("delete_bucket", { id }),
  setBucketActive: (id: string, active: boolean) =>
    call<BucketWithToken>("set_bucket_active", { id, active }),
  setBucketProxyEnabled: (id: string, enabled: boolean) =>
    call<BucketMeta>("set_bucket_proxy_enabled", { id, enabled }),
  getBucketToken: (id: string) => call<string>("get_bucket_token", { id }),
  listBucketMappings: (bucketId: string) =>
    call<BucketMapping[]>("list_bucket_mappings", { bucketId }),
  upsertBucketMapping: (req: UpsertMappingInput) =>
    call<BucketMapping>("upsert_bucket_mapping", { req }),
  deleteBucketMapping: (mappingId: string) =>
    call<void>("delete_bucket_mapping", { mappingId }),
  getSettings: () => call<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) =>
    call<void>("set_setting", { req: { key, value } }),
  showMainWindow: () => call<void>("show_main_window"),
  listPending: () => call<ClientAccessRequest[]>("list_pending"),
  respondAccess: (req: {
    requestId: string;
    accept: boolean;
    ttlMinutes?: number;
  }) => call<void>("respond_access", { req }),
  pendingCount: () => call<number>("pending_count"),
  isSignedIn: () => call<boolean>("is_signed_in"),
  listGrants: () => call<GrantRow[]>("list_grants"),
  revokeGrant: (grantId: string) =>
    call<void>("revoke_grant", { grantId }),
};

export type { RegisterProgress };
