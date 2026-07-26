import type { BucketMapping } from "@/shared/types/bucket";
import type { SecretMeta } from "@/shared/types/secret";
import { BucketMappingDetailPanel } from "@/features/buckets/mapping/detail";
import { BucketMappingListPanel } from "@/features/buckets/mapping/list";

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

  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,360px)_minmax(0,1fr)] lg:items-start">
      <BucketMappingListPanel
        mappings={mappings}
        selectedId={selectedId}
        onSelect={onSelect}
        onAdd={onAdd}
        loading={loading}
        proxyBucketEnabled={proxyBucketEnabled}
      />
      <div className="min-w-0">
        <BucketMappingDetailPanel
          bucketId={bucketId}
          mapping={draftMode ? null : selected}
          isDraft={draftMode}
          secrets={secrets}
          proxyBucketEnabled={proxyBucketEnabled}
          onDelete={() => onDelete(selected!.id)}
          onSaved={onSaved}
          onCancelDraft={onCancelDraft}
        />
      </div>
    </div>
  );
}
