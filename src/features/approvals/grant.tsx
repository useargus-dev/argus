import { useState } from "react";
import { Info } from "lucide-react";

import type { GrantRow } from "@/shared/types/client";
import { Button } from "@/shared/ui/button";

import { GrantModal } from "./modal";
import { fmtAgo, fmtExpires, stripArgs } from "@/shared/utils/time";

type Props = {
  grant: GrantRow;
  onRevoke: (id: string) => void | Promise<void>;
};

export function GrantCard({ grant, onRevoke }: Props) {
  const [showDetails, setShowDetails] = useState(false);
  const [revoking, setRevoking] = useState(false);
  const displayArgs = grant.runArgs ? stripArgs(grant.runArgs) : "";

  async function handleRevoke() {
    if (revoking) return;
    if (!window.confirm(`Revoke access for ${grant.clientLabel || grant.bucketName}?`)) {
      return;
    }
    setRevoking(true);
    try {
      await onRevoke(grant.id);
    } finally {
      setRevoking(false);
    }
  }

  return (
    <>
      <div className="flex items-center justify-between rounded-lg border border-border bg-surface p-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span
              className={`inline-block size-2 rounded-full ${grant.isActive ? "bg-green-500" : "bg-neutral-400"}`}
            />
            <span className="text-sm font-medium text-text">{grant.bucketName}</span>
            {grant.clientLabel && (
              <span className="truncate text-xs text-text-muted">— {grant.clientLabel}</span>
            )}
          </div>
          <div className="mt-1 flex flex-wrap gap-3 text-[11px] text-text-muted">
            <span>Granted {fmtAgo(grant.grantedAt)}</span>
            {grant.isActive && (
              <span className="text-green-600">
                {(() => {
                  const e = fmtExpires(grant.expiresAt);
                  return e === "expired" ? "Expired" : `Expires in ${e}`;
                })()}
              </span>
            )}
            {!grant.isActive && <span>Expired</span>}
            {grant.lastSeenAt && <span>Last used {fmtAgo(grant.lastSeenAt)}</span>}
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setShowDetails(true)}
            className="flex items-center gap-1 rounded px-2 py-1 text-[11px] text-text-muted hover:bg-surface-raised hover:text-text"
          >
            <Info size={13} />
            Details
          </button>
          <Button variant="danger" size="sm" disabled={revoking} onClick={() => void handleRevoke()}>
            {revoking ? "Revoking…" : "Revoke"}
          </Button>
        </div>
      </div>

      {showDetails && (
        <GrantModal
          grant={grant}
          displayArgs={displayArgs}
          onClose={() => setShowDetails(false)}
        />
      )}
    </>
  );
}
