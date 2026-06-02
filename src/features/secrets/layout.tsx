import type { SecretDetail, SecretMeta } from "@/shared/types/secret";
import { SecretDetailPanel } from "@/features/secrets/detail";
import { SecretListPanel } from "@/features/secrets/list";

interface VaultLayoutProps {
  secrets: SecretMeta[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  detail: SecretDetail | null;
  detailLoading: boolean;
  onEdit: () => void;
  onDelete: () => void;
}

export function VaultLayout({
  secrets,
  selectedId,
  onSelect,
  detail,
  detailLoading,
  onEdit,
  onDelete,
}: VaultLayoutProps) {
  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,360px)_minmax(0,1fr)] lg:items-start">
      <SecretListPanel
        secrets={secrets}
        selectedId={selectedId}
        onSelect={onSelect}
        detail={detail}
        detailLoading={detailLoading}
        onEdit={onEdit}
        onDelete={onDelete}
      />

      <div className="hidden min-w-0 lg:block">
        <SecretDetailPanel
          detail={detail}
          loading={detailLoading}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      </div>
    </div>
  );
}
