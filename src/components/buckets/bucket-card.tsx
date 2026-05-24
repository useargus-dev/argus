import { ArrowUpRight, Package, Trash2 } from "lucide-react";
import { Link } from "react-router-dom";

import { cn } from "../../lib/cn";
import type { BucketMeta } from "../../types/bucket";
import { SecretBadge } from "../secrets/secret-badge";
import { Switch } from "../settings/switch";
import { BucketEnvCredentials } from "./bucket-env-credentials";

interface BucketCardProps {
  bucket: BucketMeta;
  cachedToken?: string | null;
  toggling: boolean;
  onToggleActive: (active: boolean) => void;
  onDelete: () => void;
  onTokenCached: (token: string) => void;
}

export function BucketCard({
  bucket,
  cachedToken,
  toggling,
  onToggleActive,
  onDelete,
  onTokenCached,
}: BucketCardProps) {
  return (
    <article
      className={cn(
        "flex flex-col rounded-xl border border-border bg-surface p-5 transition-colors",
        !bucket.isActive && "opacity-70",
      )}
    >
      <div className="flex items-start gap-3">
        <div className="grid size-10 shrink-0 place-items-center rounded-md border border-accent/30 bg-accent/10">
          <Package className="size-5 text-accent" aria-hidden />
        </div>
        <div className="min-w-0 flex-1">
          <h3 className="truncate font-medium">{bucket.name}</h3>
          {bucket.description && (
            <p className="mt-0.5 truncate text-xs text-text-muted">
              {bucket.description}
            </p>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            {bucket.isActive ? "Active" : "Inactive"}
          </span>
          <Switch
            checked={bucket.isActive}
            disabled={toggling}
            onChange={onToggleActive}
            aria-label={
              bucket.isActive ? "Deactivate bucket" : "Activate bucket"
            }
          />
        </div>
      </div>

      <div className="mt-4 flex flex-wrap gap-1.5">
        <SecretBadge tone="accent">
          {bucket.mappingCount} mappings
        </SecretBadge>
        {bucket.activeGrantCount > 0 && (
          <SecretBadge tone="success">
            {bucket.activeGrantCount} active
          </SecretBadge>
        )}
        <SecretBadge>TTL {bucket.accessTtlMinutes}m</SecretBadge>
      </div>

      <BucketEnvCredentials
        className="mt-4"
        bucketId={bucket.id}
        cachedToken={cachedToken}
        onTokenCached={onTokenCached}
      />

      <div className="mt-4 flex items-center justify-end gap-1">
        <button
          type="button"
          aria-label="Delete bucket"
          title="Delete"
          onClick={onDelete}
          className="grid size-8 place-items-center rounded-md text-text-muted transition-colors hover:bg-danger/10 hover:text-danger"
        >
          <Trash2 className="size-4" aria-hidden />
        </button>
        <Link
          to={`/buckets/${bucket.id}`}
          aria-label="Open bucket"
          title="Open bucket"
          className="inline-flex h-8 items-center gap-1 rounded-md px-2.5 text-xs font-medium text-text-muted transition-colors hover:bg-accent/10 hover:text-accent"
        >
          Open
          <ArrowUpRight className="size-4" aria-hidden />
        </Link>
      </div>
    </article>
  );
}
