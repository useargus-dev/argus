import { Plus } from "lucide-react";

import type { BucketMapping } from "../../types/bucket";
import { Button } from "../ui/button";
import { SecretBadge } from "../secrets/secret-badge";
import { cn } from "../../lib/cn";

interface BucketMappingListPanelProps {
  mappings: BucketMapping[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  loading: boolean;
}

export function BucketMappingListPanel({
  mappings,
  selectedId,
  onSelect,
  onAdd,
  loading,
}: BucketMappingListPanelProps) {
  return (
    <div className="rounded-xl border border-border bg-surface">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <h2 className="text-sm font-semibold">Env keys</h2>
        <Button type="button" variant="secondary" size="sm" onClick={onAdd}>
          <Plus className="size-4" />
          Add
        </Button>
      </div>
      {loading ? (
        <p className="px-4 py-6 text-sm text-text-muted">Loading mappings…</p>
      ) : mappings.length === 0 ? (
        <p className="px-4 py-6 text-sm text-text-muted">No mappings yet.</p>
      ) : (
        <ul className="max-h-[min(70vh,520px)] divide-y divide-border overflow-y-auto">
          {mappings.map((m) => (
            <li key={m.id}>
              <button
                type="button"
                className={cn(
                  "flex w-full items-center justify-between gap-2 px-4 py-3 text-left transition-colors hover:bg-background/60",
                  selectedId === m.id && "bg-accent/10",
                )}
                onClick={() => onSelect(m.id)}
              >
                <div className="min-w-0">
                  <p className="truncate font-mono text-sm font-medium">{m.envLabel}</p>
                  <p className="truncate text-xs text-text-muted">
                    {m.mappingType === "secret" ? m.secretName ?? "Secret" : "Text value"}
                  </p>
                </div>
                {m.proxyEnabled && (
                  <SecretBadge tone="accent" className="shrink-0">
                    Proxy
                  </SecretBadge>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
