import { useCallback, useEffect, useState } from "react";
import { ArrowLeft, Package } from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { BucketEnvCredentials } from "../components/buckets/bucket-env-credentials";
import { BucketMappingsPanel } from "../components/buckets/bucket-mappings-panel";
import { SecretBadge } from "../components/secrets/secret-badge";
import { bridge } from "../lib/tauri-bridge";
import { toast } from "../lib/toast";
import type { BucketMeta } from "../types/bucket";

export function BucketDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [bucket, setBucket] = useState<BucketMeta | null>(null);
  const [cachedToken, setCachedToken] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const loadBucket = useCallback(async () => {
    if (!id) return;
    setLoading(true);
    try {
      const list = await bridge.listBuckets();
      const found = list.find((b) => b.id === id) ?? null;
      setBucket(found);
      if (!found) toast.error("Bucket not found");
    } catch (e) {
      toast.fromError(e, "Failed to load bucket");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    void loadBucket();
  }, [loadBucket]);

  if (loading) {
    return <p className="text-sm text-text-muted">Loading bucket…</p>;
  }

  if (!bucket || !id) {
    return (
      <div>
        <Link
          to="/buckets"
          className="inline-flex items-center gap-1 text-sm text-text-muted hover:text-text"
        >
          <ArrowLeft className="size-4" />
          Back to buckets
        </Link>
        <p className="mt-4 text-sm text-text-muted">Bucket not found.</p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-[1400px] px-2 py-2">
      <Link
        to="/buckets"
        className="inline-flex items-center gap-1 text-sm text-text-muted hover:text-text"
      >
        <ArrowLeft className="size-4" />
        App buckets
      </Link>

      <div className="mt-6 flex items-start gap-4">
        <div className="grid size-12 place-items-center rounded-lg border border-accent/30 bg-accent/10">
          <Package className="size-6 text-accent" aria-hidden />
        </div>
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">{bucket.name}</h1>
          {bucket.description && (
            <p className="mt-1 text-sm text-text-muted">{bucket.description}</p>
          )}
          <div className="mt-3 flex flex-wrap gap-1.5">
            <SecretBadge tone={bucket.isActive ? "success" : "muted"}>
              {bucket.isActive ? "Active" : "Inactive"}
            </SecretBadge>
            <SecretBadge tone="accent">
              {bucket.mappingCount} mappings
            </SecretBadge>
            <SecretBadge>TTL {bucket.accessTtlMinutes}m</SecretBadge>
          </div>
        </div>
      </div>

      <div className="mt-6 rounded-xl border border-border bg-surface p-5">
        <BucketEnvCredentials
          bucketId={bucket.id}
          cachedToken={cachedToken}
          onTokenCached={setCachedToken}
        />
      </div>

      <BucketMappingsPanel
        bucketId={id}
        onMappingsChange={loadBucket}
      />
    </div>
  );
}
