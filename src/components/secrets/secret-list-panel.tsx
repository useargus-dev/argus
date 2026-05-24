import {
  daysUntilExpiry,
  expiryBadgeTone,
  secretSubtitle,
  secretTypeIcon,
} from "../../lib/secret-utils";
import { cn } from "../../lib/cn";
import type { SecretMeta } from "../../types/secret";
import { SecretBadge } from "./secret-badge";

interface SecretListPanelProps {
  secrets: SecretMeta[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function SecretListPanel({
  secrets,
  selectedId,
  onSelect,
}: SecretListPanelProps) {
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

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-surface">
      <ul className="max-h-[70vh] divide-y divide-border overflow-auto">
        {secrets.map((s) => {
          const Icon = secretTypeIcon(s.secretType);
          const days = daysUntilExpiry(s.expiresAt);
          const expiryTone = days !== null ? expiryBadgeTone(days) : null;
          const selected = s.id === selectedId;

          return (
            <li key={s.id}>
              <button
                type="button"
                onClick={() => onSelect(s.id)}
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
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
