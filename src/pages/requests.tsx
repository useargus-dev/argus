import { useCallback, useEffect, useState } from "react";

import { bridge } from "../lib/tauri-bridge";
import { toast } from "../lib/toast";
import { useTauriEvent } from "../hooks/use-tauri-event";
import type { ClientAccessRequest } from "../types/client";
import { Button } from "../components/ui/button";

const TTL_OPTIONS = [15, 60, 180, 480] as const;

function timeAgo(iso: string) {
  const ms = Date.now() - new Date(iso).getTime();
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s ago`;
  return `${Math.floor(s / 60)}m ago`;
}

function RequestCard({
  request,
  onResolved,
}: {
  request: ClientAccessRequest;
  onResolved: () => void;
}) {
  const [ttl, setTtl] = useState(request.accessTtlMinutes || 60);
  const [busy, setBusy] = useState(false);

  const respond = useCallback(
    async (accept: boolean) => {
      setBusy(true);
      try {
        await bridge.respondToClientAccess({
          requestId: request.requestId,
          accept,
          ttlMinutes: accept ? ttl : undefined,
        });
        onResolved();
      } catch (e) {
        toast.fromError(e, "Action failed");
      } finally {
        setBusy(false);
      }
    },
    [request.requestId, ttl, onResolved],
  );

  return (
    <div className="rounded-lg border border-border bg-surface p-4 shadow-sm">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium text-text">
            {request.processName}{" "}
            <span className="text-text-muted font-normal">
              wants access to
            </span>{" "}
            <span className="text-accent">{request.bucketName}</span>
          </p>
          <p className="mt-0.5 text-[11px] text-text-muted">
            {timeAgo(request.createdAt)} · pid {request.pid}
          </p>
        </div>
      </div>

      <dl className="mt-3 space-y-1.5 text-[11px]">
        <div className="flex gap-2">
          <dt className="w-16 shrink-0 text-text-muted">CWD</dt>
          <dd className="min-w-0 break-all font-mono text-text">
            {request.cwd}
            {!request.cwdVerified && (
              <span className="ml-1 text-yellow-500">(unverified)</span>
            )}
          </dd>
        </div>
        <div className="flex gap-2">
          <dt className="w-16 shrink-0 text-text-muted">Exe</dt>
          <dd className="min-w-0 break-all font-mono text-text">
            {request.exePath}
          </dd>
        </div>
        {request.gitRemote && (
          <div className="flex gap-2">
            <dt className="w-16 shrink-0 text-text-muted">Git</dt>
            <dd className="min-w-0 break-all font-mono text-text">
              {request.gitRemote}
            </dd>
          </div>
        )}
        {request.runArgs && (
          <div className="flex gap-2">
            <dt className="w-16 shrink-0 text-text-muted">Args</dt>
            <dd className="min-w-0 truncate font-mono text-text-muted">
              {request.runArgs}
            </dd>
          </div>
        )}
      </dl>

      <div className="mt-3 flex items-center gap-2">
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

      <div className="mt-3 flex gap-2">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={busy}
          onClick={() => respond(false)}
        >
          Deny
        </Button>
        <Button
          type="button"
          variant="primary"
          size="sm"
          disabled={busy}
          onClick={() => respond(true)}
        >
          Allow
        </Button>
      </div>
    </div>
  );
}

export function RequestsPage() {
  const [requests, setRequests] = useState<ClientAccessRequest[]>([]);

  const refresh = useCallback(() => {
    bridge
      .listPendingClientAccess()
      .then(setRequests)
      .catch(() => {});
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useTauriEvent<ClientAccessRequest>("client-access-requested", () => {
    refresh();
  });

  if (requests.length === 0) {
    return (
      <div className="flex h-dvh items-center justify-center bg-bg p-4">
        <p className="text-sm text-text-muted">No pending requests</p>
      </div>
    );
  }

  return (
    <div className="flex h-dvh flex-col bg-bg">
      <header className="shrink-0 border-b border-border px-4 py-3">
        <h1 className="text-sm font-semibold text-text">
          Access Requests{" "}
          <span className="ml-1 text-text-muted">({requests.length})</span>
        </h1>
      </header>
      <div className="flex-1 overflow-y-auto p-3 space-y-3">
        {requests.map((r) => (
          <RequestCard key={r.requestId} request={r} onResolved={refresh} />
        ))}
      </div>
    </div>
  );
}
