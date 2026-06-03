import { ChevronDown } from "lucide-react";
import { useEffect, useState } from "react";

import {
  daysUntilExpiry,
  expiryBadgeTone,
  secretSubtitle,
  secretTypeIcon,
} from "@/core/secrets";
import { cn } from "@/core/cn";
import type { SecretDetail, SecretMeta } from "@/shared/types/secret";
import { SecretBadge } from "@/features/secrets/badge";
import { SecretDetailPanel } from "@/features/secrets/detail";

interface SecretListPanelProps {
  secrets: SecretMeta[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  detail?: SecretDetail | null;
  detailLoading?: boolean;
  onEdit?: () => void;
  onDelete?: () => void;
}

export function SecretListPanel({
  secrets,
  selectedId,
  onSelect,
  detail = null,
  detailLoading = false,
  onEdit,
  onDelete,
}: SecretListPanelProps) {
  const [expanded, setExpanded] = useState(true);

  useEffect(() => {
    if (selectedId) setExpanded(true);
  }, [selectedId]);

  function handleRowClick(id: string) {
    if (id === selectedId) {
      setExpanded((open) => !open);
    } else {
      onSelect(id);
      setExpanded(true);
    }
  }

  if (secrets.length === 0) {
    return (
      <div className="rounded-xl border border-border bg-surface p-8 text-center">
        <p className="text-sm text-text-muted">
          No secrets match your filters. Try adjusting search or filters, or add a
          new secret.
        </p>
      </div>
    );
  }

  const showInlineDetail = onEdit != null && onDelete != null;

  return (
    <div className="rounded-xl border border-border bg-surface lg:overflow-hidden">
      <ul className="divide-y divide-border lg:max-h-[calc(100vh-12rem)] lg:overflow-y-auto">
        {secrets.map((s) => {
          const Icon = secretTypeIcon(s.secretType);
          const days = daysUntilExpiry(s.expiresAt);
          const expiryTone = days !== null ? expiryBadgeTone(days) : null;
          const selected = s.id === selectedId;
          const isOpen = selected && expanded;

          return (
            <li key={s.id}>
              <button
                type="button"
                onClick={() => handleRowClick(s.id)}
                {...(selected
                  ? { "aria-expanded": isOpen ? "true" : "false" }
                  : {})}
                className={cn(
                  "flex w-full items-center gap-3 px-4 py-3 text-left transition-colors",
                  selected
                    ? "bg-surface-raised"
                    : "hover:bg-surface-raised/50",
                )}
              >
                <div className="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-surface-raised">
                  <Icon className="size-4 text-text-muted" aria-hidden />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{s.name}</div>
                  <div className="truncate text-xs text-text-muted">
                    {secretSubtitle(s)}
                  </div>
                </div>
                {expiryTone && days !== null && (
                  <SecretBadge tone={expiryTone}>
                    {days < 0 ? "expired" : `${days}d`}
                  </SecretBadge>
                )}
                {selected && showInlineDetail && (
                  <ChevronDown
                    className={cn(
                      "size-4 shrink-0 text-text-muted transition-transform lg:hidden",
                      isOpen && "rotate-180",
                    )}
                    aria-hidden
                  />
                )}
              </button>
              {showInlineDetail && isOpen && (
                <div className="border-t border-border bg-surface-raised/40 lg:hidden">
                  <SecretDetailPanel
                    embedded
                    detail={detail}
                    loading={detailLoading}
                    onEdit={onEdit}
                    onDelete={onDelete}
                  />
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
