import { useCallback, useEffect, useState } from "react";
import { Plus } from "lucide-react";

import { BucketCard } from "../components/buckets/bucket-card";
import { CreateBucketDialog } from "../components/buckets/create-bucket-dialog";
import { Button } from "../components/ui/button";
import { bridge, BridgeError } from "../lib/tauri-bridge";
import { toast } from "../lib/toast";
import { useAuthStore } from "../state/auth-store";
import type { BucketMeta } from "../types/bucket";

export function BucketsPage() {
  const appUnlocked = useAuthStore((s) => s.scopes?.app ?? false);
  const [buckets, setBuckets] = useState<BucketMeta[]>([]);
  const [tokenCache, setTokenCache] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [togglingId, setTogglingId] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [creating, setCreating] = useState(false);

  const loadBuckets = useCallback(async () => {
    setLoading(true);
    try {
      const list = await bridge.listBuckets();
      setBuckets(list);
    } catch (e) {
      if (e instanceof BridgeError && e.code === "APP_LOCKED") return;
      toast.fromError(e, "Failed to load buckets");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!appUnlocked) {
      setBuckets([]);
      return;
    }
    void loadBuckets();
  }, [appUnlocked, loadBuckets]);

  function cacheToken(id: string, token: string) {
    setTokenCache((prev) => ({ ...prev, [id]: token }));
  }

  async function handleToggle(id: string, active: boolean) {
    setTogglingId(id);
    try {
      const result = await bridge.setBucketActive(id, active);
      cacheToken(id, result.token);
      toast.success(
        active
          ? "Bucket activated — new token issued"
          : "Bucket deactivated — new token issued",
      );
      void loadBuckets();
    } catch (e) {
      toast.fromError(e, "Failed to update bucket");
    } finally {
      setTogglingId(null);
    }
  }

  async function handleDelete(id: string, name: string) {
    if (!confirm(`Delete bucket “${name}”? This cannot be undone.`)) return;
    try {
      await bridge.deleteBucket(id);
      setTokenCache((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      toast.success("Bucket deleted");
      void loadBuckets();
    } catch (e) {
      toast.fromError(e, "Failed to delete bucket");
    }
  }

  async function handleCreate(input: { name: string; description?: string }) {
    setCreating(true);
    try {
      const created = await bridge.createBucket(input);
      cacheToken(created.id, created.token);
      toast.success("Bucket created");
      setDialogOpen(false);
      void loadBuckets();
    } catch (e) {
      toast.fromError(e, "Failed to create bucket");
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className="mx-auto max-w-[1400px] px-2 py-2">
      <div className="mb-6 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight text-text">
            App buckets
          </h1>
          <p className="mt-1 text-sm text-text-muted">
            Bundles of secrets that you grant to running applications.
          </p>
        </div>
        {appUnlocked && (
          <Button
            type="button"
            variant="primary"
            className="h-10 gap-2 text-sm"
            onClick={() => setDialogOpen(true)}
          >
            <Plus className="size-4" aria-hidden />
            Create bucket
          </Button>
        )}
      </div>

      {!appUnlocked ? (
        <p className="text-sm text-text-muted">
          Unlock the app to manage buckets.
        </p>
      ) : loading ? (
        <p className="text-sm text-text-muted">Loading buckets…</p>
      ) : buckets.length === 0 ? (
        <div className="rounded-xl border border-border bg-surface p-10 text-center">
          <p className="text-sm text-text-muted">
            No buckets yet. Create one to map secrets for your apps.
          </p>
        </div>
      ) : (
        <div className="flex flex-col gap-4">
          {buckets.map((bucket) => (
            <BucketCard
              key={bucket.id}
              bucket={bucket}
              cachedToken={tokenCache[bucket.id]}
              toggling={togglingId === bucket.id}
              onToggleActive={(active) => handleToggle(bucket.id, active)}
              onDelete={() => handleDelete(bucket.id, bucket.name)}
              onTokenCached={(token) => cacheToken(bucket.id, token)}
            />
          ))}
        </div>
      )}

      <CreateBucketDialog
        open={dialogOpen}
        saving={creating}
        onClose={() => setDialogOpen(false)}
        onCreate={handleCreate}
      />
    </div>
  );
}
