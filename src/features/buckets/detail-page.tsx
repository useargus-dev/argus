import { useCallback, useEffect, useState } from "react";
import { ArrowLeft, Package } from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { BucketEnvAccordion } from "@/features/buckets/env/accordion";
import { BucketLayout } from "@/features/buckets/layout";
import { BucketProxySettings } from "@/features/buckets/proxy";
import { SecretBadge } from "@/features/secrets/badge";
import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import type { BucketMapping, BucketMeta } from "@/shared/types/bucket";
import type { SecretMeta } from "@/shared/types/secret";

export function BucketDetailPage() {
  const { id } = useParams<{ id: string }>();
  const [bucket, setBucket] = useState<BucketMeta | null>(null);
  const [mappings, setMappings] = useState<BucketMapping[]>([]);
  const [secrets, setSecrets] = useState<SecretMeta[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draftMode, setDraftMode] = useState(false);
  const [cachedToken, setCachedToken] = useState<string | null>(null);
  const [initialLoading, setInitialLoading] = useState(true);
  const [mappingsLoading, setMappingsLoading] = useState(true);

  const loadBucket = useCallback(
    async (opts?: { silent?: boolean }) => {
      if (!id) return;
      if (!opts?.silent) setInitialLoading(true);
      try {
        const list = await bridge.listBuckets();
        const found = list.find((b) => b.id === id) ?? null;
        setBucket(found);
        if (!found) toast.error("Bucket not found");
      } catch (e) {
        toast.fromError(e, "Failed to load bucket");
      } finally {
        if (!opts?.silent) setInitialLoading(false);
      }
    },
    [id],
  );

  const loadMappings = useCallback(
    async (opts?: { silent?: boolean }) => {
      if (!id) return;
      if (!opts?.silent) setMappingsLoading(true);
      try {
        const [mapList, secretList] = await Promise.all([
          bridge.listBucketMappings(id),
          bridge.searchSecrets(),
        ]);
        setMappings(mapList);
        setSecrets(secretList);
      } catch (e) {
        toast.fromError(e, "Failed to load mappings");
      } finally {
        if (!opts?.silent) setMappingsLoading(false);
      }
    },
    [id],
  );

  const handleMappingSaved = useCallback((saved: BucketMapping) => {
    setDraftMode(false);
    setSelectedId(saved.id);
    setMappings((prev) => {
      const byId = prev.findIndex((m) => m.id === saved.id);
      const byEnv = prev.findIndex((m) => m.envLabel === saved.envLabel);
      const idx = byId >= 0 ? byId : byEnv;
      let next: BucketMapping[];
      if (idx >= 0) {
        next = [...prev];
        next[idx] = saved;
      } else {
        next = [...prev, saved].sort((a, b) =>
          a.envLabel.localeCompare(b.envLabel, undefined, { sensitivity: "base" }),
        );
      }
      setBucket((b) => (b ? { ...b, mappingCount: next.length } : b));
      return next;
    });
  }, []);

  useEffect(() => {
    void loadBucket();
    void loadMappings();
  }, [loadBucket, loadMappings]);

  useEffect(() => {
    if (mappings.length === 0) {
      setSelectedId(null);
      return;
    }
    if (!selectedId && !draftMode) {
      setSelectedId(mappings[0]?.id ?? null);
    }
  }, [mappings, selectedId, draftMode]);

  async function handleDeleteMapping(mappingId: string) {
    if (!confirm("Delete this mapping?")) return;
    try {
      await bridge.deleteBucketMapping(mappingId);
      if (selectedId === mappingId) setSelectedId(null);
      await loadMappings({ silent: true });
      await loadBucket({ silent: true });
    } catch (e) {
      toast.fromError(e, "Failed to delete mapping");
    }
  }

  if (initialLoading && !bucket) {
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
            <SecretBadge tone="accent">{bucket.mappingCount} mappings</SecretBadge>
            {bucket.proxyEnabled && bucket.proxyPort != null && (
              <SecretBadge tone="accent">Proxy :{bucket.proxyPort}</SecretBadge>
            )}
            <SecretBadge>TTL {bucket.accessTtlMinutes}m</SecretBadge>
          </div>
        </div>
      </div>

      <div className="mt-6 space-y-4">
        <BucketProxySettings bucket={bucket} onBucketChange={setBucket} />

        <BucketEnvAccordion
          bucketId={bucket.id}
          cachedToken={cachedToken}
          onTokenCached={setCachedToken}
        />

        <BucketLayout
          bucketId={id}
          mappings={mappings}
          secrets={secrets}
          selectedId={selectedId}
          draftMode={draftMode}
          proxyBucketEnabled={bucket.proxyEnabled}
          loading={mappingsLoading}
          onSelect={(mid) => {
            setDraftMode(false);
            setSelectedId(mid);
          }}
          onAdd={() => {
            setDraftMode(true);
            setSelectedId(null);
          }}
          onDelete={(mid) => void handleDeleteMapping(mid)}
          onSaved={handleMappingSaved}
          onCancelDraft={() => setDraftMode(false)}
        />
      </div>
    </div>
  );
}
