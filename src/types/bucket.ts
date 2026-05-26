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
  mappingType: "secret" | "text";
  secretId?: string | null;
  secretName?: string | null;
  secretType?: string | null;
  textValue?: string | null;
  createdAt: string;
}

export interface UpsertMappingInput {
  bucketId: string;
  envLabel: string;
  mappingType: "secret" | "text";
  secretId?: string;
  textValue?: string;
}
