export interface BucketMeta {
  id: string;
  name: string;
  description: string | null;
  isActive: boolean;
  accessTtlMinutes: number;
  refreshTtlMinutes: number | null;
  sessionTtlMinutes: number;
  mappingCount: number;
  activeGrantCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface BucketWithToken extends BucketMeta {
  token: string;
}

export interface CreateBucketInput {
  name: string;
  description?: string;
}

export interface BucketMapping {
  id: string;
  bucketId: string;
  envLabel: string;
  secretId: string;
  secretName: string;
  secretType: string;
  createdAt: string;
}

export interface UpsertMappingInput {
  bucketId: string;
  envLabel: string;
  secretId: string;
}
