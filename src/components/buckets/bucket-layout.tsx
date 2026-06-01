import { useMemo } from "react";

import type { BucketMapping } from "../../types/bucket";
import { collectBucketProxyTokens } from "../../lib/proxy-token";
import type { SecretMeta } from "../../types/secret";
import { BucketMappingDetailPanel } from "./bucket-mapping-detail-panel";
import { BucketMappingListPanel } from "./bucket-mapping-list-panel";

interface BucketLayoutProps {
  bucketId: string;
  mappings: BucketMapping[];
  secrets: SecretMeta[];
  selectedId: string | null;
  draftMode: boolean;
  proxyBucketEnabled: boolean;
  loading: boolean;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onDelete: (id: string) => void;
  onSaved: (saved: BucketMapping) => void;
  onCancelDraft: () => void;
}

export function BucketLayout({
  bucketId,
  mappings,
  secrets,
  selectedId,
  draftMode,
  proxyBucketEnabled,
  loading,
  onSelect,
  onAdd,
  onDelete,
  onSaved,
  onCancelDraft,
}: BucketLayoutProps) {
  const selected = mappings.find((m) => m.id === selectedId) ?? null;
  const bucketProxyTokens = useMemo(
    () => collectBucketProxyTokens(mappings),
    [mappings],
  );

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,360px)_minmax(0,1fr)] lg:items-start">
      <BucketMappingListPanel
        mappings={mappings}
        selectedId={selectedId}
        onSelect={onSelect}
        onAdd={onAdd}
        loading={loading}
      />
      <div className="min-w-0">
        <BucketMappingDetailPanel
          bucketId={bucketId}
          mapping={draftMode ? null : selected}
          isDraft={draftMode}
          secrets={secrets}
          proxyBucketEnabled={proxyBucketEnabled}
          bucketProxyTokens={bucketProxyTokens}
          onDelete={() => onDelete(selected!.id)}
          onSaved={onSaved}
          onCancelDraft={onCancelDraft}
        />
      </div>
    </div>
  );
}
