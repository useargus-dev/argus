export type SecretType =
  | "api_key"
  | "access_token"
  | "credential"
  | "recovery_codes"
  | "ssh_key"
  | "certificate"
  | "connection_string"
  | "note"
  | "password";

export interface SecretMeta {
  id: string;
  name: string;
  secretType: SecretType;
  organization: string | null;
  environment: string | null;
  description: string | null;
  tags: string[];
  expiresAt: string | null;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface SecretDetail extends SecretMeta {
  value: Record<string, string>;
}

export interface SecretWriteInput {
  name: string;
  secretType: SecretType;
  organization?: string;
  environment?: string;
  description?: string;
  tags?: string[];
  expiresAt?: string;
  value: Record<string, string>;
}
