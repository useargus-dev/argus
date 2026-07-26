import { useEffect } from "react";
import { X } from "lucide-react";

import type { GrantRow } from "@/shared/types/client";
import { Button } from "@/shared/ui/button";

type Props = {
  grant: GrantRow;
  displayArgs: string;
  onClose: () => void;
};

export function GrantModal({ grant, displayArgs, onClose }: Props) {
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div
        className="absolute inset-0"
        role="presentation"
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="grant-modal-title"
        className="relative w-full max-w-md rounded-lg border border-border bg-surface p-5 shadow-xl"
      >
        <div className="flex items-center justify-between">
          <h3 id="grant-modal-title" className="text-sm font-semibold text-text">
            Grant Details
          </h3>
          <button
            type="button"
            onClick={onClose}
            className="rounded p-1 text-text-muted hover:bg-surface-raised hover:text-text"
            aria-label="Close"
          >
            <X size={16} />
          </button>
        </div>

        <dl className="mt-4 space-y-3 text-xs">
          {grant.cwd && (
            <div>
              <dt className="font-medium text-text-muted">Folder</dt>
              <dd className="mt-0.5 break-all font-mono text-text">{grant.cwd}</dd>
            </div>
          )}
          {grant.exePath && (
            <div>
              <dt className="font-medium text-text-muted">Executable</dt>
              <dd className="mt-0.5 break-all font-mono text-text">{grant.exePath}</dd>
            </div>
          )}
          {grant.gitRemote && (
            <div>
              <dt className="font-medium text-text-muted">Git Remote</dt>
              <dd className="mt-0.5 break-all font-mono text-text">{grant.gitRemote}</dd>
            </div>
          )}
          {displayArgs && (
            <div>
              <dt className="font-medium text-text-muted">Arguments</dt>
              <dd className="mt-0.5 break-all font-mono text-text">{displayArgs}</dd>
            </div>
          )}
          <div>
            <dt className="font-medium text-text-muted">Granted At</dt>
            <dd className="mt-0.5 text-text">{new Date(grant.grantedAt).toLocaleString()}</dd>
          </div>
          <div>
            <dt className="font-medium text-text-muted">Expires At</dt>
            <dd className="mt-0.5 text-text">{new Date(grant.expiresAt).toLocaleString()}</dd>
          </div>
          {grant.lastSeenAt && (
            <div>
              <dt className="font-medium text-text-muted">Last Used</dt>
              <dd className="mt-0.5 text-text">{new Date(grant.lastSeenAt).toLocaleString()}</dd>
            </div>
          )}
          {!grant.cwd && !grant.exePath && !grant.gitRemote && !displayArgs && (
            <div>
              <dd className="text-text-muted">
                No process details available for this grant (granted before details tracking was added).
              </dd>
            </div>
          )}
        </dl>

        <div className="mt-5 flex justify-end">
          <Button variant="ghost" size="sm" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}
