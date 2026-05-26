export type SecondFactorType = "totp" | "biometric";

export interface UserProfile {
  email: string;
  username: string;
  firstName: string;
  lastName: string;
}

export interface ScopeStatus {
  app: boolean;
  vault: boolean;
  buckets: boolean;
  vaultExpiresAt: string | null;
  bucketsExpiresAt: string | null;
}

export interface TotpSetup {
  secret: string;
  otpauthUri: string;
}

export interface RegisterProgress {
  step: string;
  status: "running" | "done" | "error";
  message?: string;
}

export interface AuthErrorPayload {
  code: string;
  message: string;
  secondFactorType?: SecondFactorType;
}

export const PROVISIONING_STEPS: { key: string; label: string }[] = [
  { key: "validate_draft", label: "Validating account" },
  { key: "create_data_dir", label: "Preparing secure storage" },
  { key: "open_database", label: "Creating encrypted database" },
  { key: "run_migrations", label: "Applying schema" },
  { key: "derive_keys", label: "Deriving encryption keys" },
  { key: "persist_user", label: "Registering user" },
  { key: "open_session", label: "Starting session" },
  { key: "complete", label: "Account secured" },
];
