import { useCallback, useState } from "react";

import type { ClientAccessRequest } from "@/shared/types/client";
import { Button } from "@/shared/ui/button";

import { fmtAgo, stripArgs } from "@/shared/utils/time";

const TTL_OPTIONS = [15, 60, 180, 480] as const;

type Props = {
  request: ClientAccessRequest;
  onRespond: (
    requestId: string,
    accept: boolean,
    ttlMinutes?: number,
  ) => Promise<void>;
};

export function PendingCard({ request, onRespond }: Props) {
  const [ttl, setTtl] = useState(request.accessTtlMinutes || 60);
  const [busy, setBusy] = useState(false);

  const handle = useCallback(
    async (accept: boolean) => {
      setBusy(true);
      try {
        await onRespond(request.requestId, accept, accept ? ttl : undefined);
      } finally {
        setBusy(false);
      }
    },
    [request.requestId, ttl, onRespond],
  );

  const displayArgs = request.runArgs ? stripArgs(request.runArgs) : "";

  return (
    <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/5 p-4">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-text">
            {request.processName}{" "}
            <span className="font-normal text-text-muted">wants access to</span>{" "}
            <span className="text-accent">{request.bucketName}</span>
          </p>
          <p className="mt-0.5 text-[11px] text-text-muted">
            {fmtAgo(request.createdAt)} · pid {request.pid}
          </p>
        </div>
      </div>

      <dl className="mt-3 space-y-1.5 text-[11px]">
        <div className="flex gap-2">
          <dt className="w-12 shrink-0 text-text-muted">Folder</dt>
          <dd className="min-w-0 break-all font-mono text-text">
            {request.cwd}
            {!request.cwdVerified && (
              <span className="ml-1 text-yellow-500">(unverified)</span>
            )}
          </dd>
        </div>
        <div className="flex gap-2">
          <dt className="w-12 shrink-0 text-text-muted">Exe</dt>
          <dd className="min-w-0 break-all font-mono text-text">{request.exePath}</dd>
        </div>
        {request.gitRemote && (
          <div className="flex gap-2">
            <dt className="w-12 shrink-0 text-text-muted">Git</dt>
            <dd className="min-w-0 break-all font-mono text-text">{request.gitRemote}</dd>
          </div>
        )}
        {displayArgs && (
          <div className="flex gap-2">
            <dt className="w-12 shrink-0 text-text-muted">Args</dt>
            <dd className="min-w-0 break-all font-mono text-text-muted">{displayArgs}</dd>
          </div>
        )}
      </dl>

      <div className="mt-3 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-semibold uppercase tracking-wider text-text-muted">
            TTL
          </span>
          {TTL_OPTIONS.map((m) => (
            <button
              key={m}
              type="button"
              disabled={busy}
              onClick={() => setTtl(m)}
              className={
                ttl === m
                  ? "rounded border border-accent bg-accent/10 px-2 py-0.5 text-[10px] font-medium text-accent"
                  : "rounded border border-border px-2 py-0.5 text-[10px] text-text-muted hover:bg-surface-raised"
              }
            >
              {m < 60 ? `${m}m` : `${m / 60}h`}
            </button>
          ))}
        </div>
        <div className="flex gap-2">
          <Button variant="ghost" size="sm" disabled={busy} onClick={() => handle(false)}>
            Deny
          </Button>
          <Button variant="primary" size="sm" disabled={busy} onClick={() => handle(true)}>
            Allow
          </Button>
        </div>
      </div>
    </div>
  );
}
