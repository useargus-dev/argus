import { useEffect, useState } from "react";
import { Copy, Eye, EyeOff, Pencil, Trash2 } from "lucide-react";

import {
  daysUntilExpiry,
  formatDate,
  readSecretValue,
  secretTypeIcon,
  secretTypeLabel,
} from "../../lib/secret-utils";
import { toast } from "../../lib/toast";
import type { SecretDetail } from "../../types/secret";
import { Button } from "../ui/button";
import { SecretBadge } from "./secret-badge";

const REVEAL_MS = 60_000;

interface SecretDetailPanelProps {
  detail: SecretDetail | null;
  loading: boolean;
  onEdit: () => void;
  onDelete: () => void;
}

export function SecretDetailPanel({
  detail,
  loading,
  onEdit,
  onDelete,
}: SecretDetailPanelProps) {
  const [revealed, setRevealed] = useState(false);
  const plain = detail ? readSecretValue(detail.value) : "";

  useEffect(() => {
    setRevealed(false);
  }, [detail?.id]);

  useEffect(() => {
    if (!revealed) return;
    const id = window.setTimeout(() => setRevealed(false), REVEAL_MS);
    return () => window.clearTimeout(id);
  }, [revealed, detail?.id]);

  if (loading) {
    return (
      <div className="rounded-xl border border-border bg-surface p-6">
        <p className="text-sm text-text-muted">Loading secret…</p>
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="flex min-h-[320px] items-center justify-center rounded-xl border border-border bg-surface p-6">
        <p className="text-sm text-text-muted">
          Select a secret from the list or add a new one.
        </p>
      </div>
    );
  }

  const Icon = secretTypeIcon(detail.secretType);
  const days = daysUntilExpiry(detail.expiresAt);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(plain);
      toast.success("Copied to clipboard");
    } catch {
      toast.error("Could not copy to clipboard");
    }
  }

  return (
    <div className="rounded-xl border border-border bg-surface p-6">
      <div className="flex h-full flex-col">
        <div className="flex items-start gap-4">
          <div className="grid size-12 shrink-0 place-items-center rounded-lg border border-accent/30 bg-accent/10">
            <Icon className="size-5 text-accent" aria-hidden />
          </div>
          <div className="min-w-0 flex-1">
            <h2 className="text-lg font-semibold leading-tight">{detail.name}</h2>
            <div className="mt-2 flex flex-wrap items-center gap-1.5">
              <SecretBadge tone="accent">{secretTypeLabel(detail.secretType)}</SecretBadge>
              {detail.organization && (
                <SecretBadge>{detail.organization}</SecretBadge>
              )}
              {detail.environment && (
                <SecretBadge>{detail.environment}</SecretBadge>
              )}
              {detail.tags.map((tag) => (
                <SecretBadge key={tag} prefix="#">
                  {tag}
                </SecretBadge>
              ))}
            </div>
          </div>
          <div className="flex gap-1">
            <Button
              type="button"
              variant="ghost"
              className="h-8 px-3 text-xs"
              onClick={onEdit}
            >
              <Pencil className="size-3.5" aria-hidden />
              Edit
            </Button>
            <Button
              type="button"
              variant="danger"
              className="h-8 px-3 text-xs"
              onClick={onDelete}
              aria-label="Delete secret"
            >
              <Trash2 className="size-3.5" aria-hidden />
            </Button>
          </div>
        </div>

        {detail.description && (
          <p className="mt-4 text-sm text-text-muted">{detail.description}</p>
        )}

        <div className="mt-5">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-text-muted">
              Value
            </span>
            <div className="flex items-center gap-1">
              <Button
                type="button"
                variant="ghost"
                className="h-8 px-3 text-xs"
                onClick={() => setRevealed((v) => !v)}
              >
                {revealed ? (
                  <EyeOff className="size-3.5" aria-hidden />
                ) : (
                  <Eye className="size-3.5" aria-hidden />
                )}
                {revealed ? "Hide" : "Reveal"}
              </Button>
              <Button
                type="button"
                variant="ghost"
                className="h-8 px-3 text-xs"
                onClick={handleCopy}
                disabled={!plain}
              >
                <Copy className="size-3.5" aria-hidden />
                Copy
              </Button>
            </div>
          </div>
          <pre className="min-h-[88px] overflow-auto whitespace-pre-wrap break-all rounded-md border border-border bg-surface-raised p-3 font-mono text-xs">
            {revealed ? plain || "—" : "•".repeat(Math.min(Math.max(plain.length, 32), 48))}
          </pre>
          <p className="mt-2 text-[10px] text-text-muted">
            Auto-hides after 60s
          </p>
        </div>

        <div className="mt-5 grid grid-cols-2 gap-4 text-xs">
          <div>
            <div className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-text-muted">
              Expires
            </div>
            <div className="text-sm">
              {detail.expiresAt ? (
                <span>
                  {formatDate(detail.expiresAt)}
                  {days !== null && (
                    <span className="ml-1 text-text-muted">
                      ({days < 0 ? "expired" : `${days}d left`})
                    </span>
                  )}
                </span>
              ) : (
                <span className="text-text-muted">No expiry</span>
              )}
            </div>
          </div>
          <div>
            <div className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-text-muted">
              Last updated
            </div>
            <div className="text-sm">{formatDate(detail.updatedAt)}</div>
          </div>
        </div>
      </div>
    </div>
  );
}
