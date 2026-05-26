export interface ClientAccessRequest {
  requestId: string;
  bucketId: string;
  bucketName: string;
  fingerprint: string;
  pid: number;
  exePath: string;
  cwd: string;
  cwdVerified: boolean;
  runArgs: string;
  gitRemote?: string | null;
  processName: string;
  machineId: string;
  accessTtlMinutes: number;
  createdAt: string;
}

export interface GrantRow {
  id: string;
  bucketId: string;
  bucketName: string;
  fingerprint: string;
  clientLabel?: string | null;
  grantedAt: string;
  expiresAt: string;
  lastSeenAt?: string | null;
  isActive: boolean;
}
