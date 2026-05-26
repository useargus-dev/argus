import { useCallback, useEffect, useState } from "react";

import { bridge } from "../lib/tauri-bridge";
import { toast } from "../lib/toast";
import { useTauriEvent } from "../hooks/use-tauri-event";
import type { ClientAccessRequest, GrantRow } from "../types/client";
import { Button } from "../components/ui/button";
import { Info, X } from "lucide-react";

const TTL_OPTIONS = [15, 60, 180, 480] as const;

function timeAgo(iso: string) {
  const ms = Date.now() - new Date(iso).getTime();
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

function expiresIn(iso: string) {
  const ms = new Date(iso).getTime() - Date.now();
  if (ms <= 0) return "expired";
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`;
}

function stripSensitiveArgs(args: string): string {
  return args
    .replace(/--bucket-id\s+\S+/gi, "")
    .replace(/--token\s+\S+/gi, "")
    .replace(/\s{2,}/g, " ")
    .trim();
}

export function ApprovalsPage() {
  const [grants, setGrants] = useState<GrantRow[]>([]);
  const [pending, setPending] = useState<ClientAccessRequest[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const [grantList, pendingList] = await Promise.all([
        bridge.listGrants(),
        bridge.listPendingClientAccess(),
      ]);
      setGrants(grantList);
      setPending(pendingList);
    } catch {
      /* locked or not signed in */
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useTauriEvent("client-access-requested", () => {
    refresh();
  });

  useTauriEvent("client-access-resolved", () => {
    refresh();
  });

  const revoke = useCallback(
    async (id: string) => {
      try {
        await bridge.revokeGrant(id);
        toast.success("Grant revoked");
        refresh();
      } catch (e) {
        toast.fromError(e, "Failed to revoke grant");
      }
    },
    [refresh],
  );

  const respond = useCallback(
    async (requestId: string, accept: boolean, ttlMinutes?: number) => {
      try {
        await bridge.respondToClientAccess({ requestId, accept, ttlMinutes });
        refresh();
      } catch (e) {
        toast.fromError(e, "Action failed");
      }
    },
    [refresh],
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <p className="text-sm text-text-muted">Loading...</p>
      </div>
    );
  }

  const active = grants.filter((g) => g.isActive);
  const expired = grants.filter((g) => !g.isActive);

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-text">Approvals</h1>
        <p className="mt-1 text-sm text-text-muted">
          Pending requests, active grants, and expired access to your buckets.
        </p>
      </div>

      {pending.length === 0 && grants.length === 0 && (
        <div className="rounded-lg border border-border bg-surface p-8 text-center">
          <p className="text-sm text-text-muted">No approvals yet</p>
          <p className="mt-1 text-xs text-text-muted">
            When an IPC client requests access to a bucket, it will appear here.
          </p>
        </div>
      )}

      {pending.length > 0 && (
        <section>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-yellow-600">
            Pending ({pending.length})
          </h2>
          <div className="space-y-2">
            {pending.map((r) => (
              <PendingCard key={r.requestId} request={r} onRespond={respond} />
            ))}
          </div>
        </section>
      )}

      {active.length > 0 && (
        <section>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-text-muted">
            Active ({active.length})
          </h2>
          <div className="space-y-2">
            {active.map((g) => (
              <GrantCard key={g.id} grant={g} onRevoke={revoke} />
            ))}
          </div>
        </section>
      )}

      {expired.length > 0 && (
        <section>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-text-muted">
            Expired ({expired.length})
          </h2>
          <div className="space-y-2 opacity-60">
            {expired.map((g) => (
              <GrantCard key={g.id} grant={g} onRevoke={revoke} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function PendingCard({
  request,
  onRespond,
}: {
  request: ClientAccessRequest;
  onRespond: (requestId: string, accept: boolean, ttlMinutes?: number) => void;
}) {
  const [ttl, setTtl] = useState(request.accessTtlMinutes || 60);
  const [busy, setBusy] = useState(false);

  const handle = useCallback(
    async (accept: boolean) => {
      setBusy(true);
      onRespond(request.requestId, accept, accept ? ttl : undefined);
    },
    [request.requestId, ttl, onRespond],
  );

  const displayArgs = request.runArgs ? stripSensitiveArgs(request.runArgs) : "";

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
            {timeAgo(request.createdAt)} · pid {request.pid}
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
          <dd className="min-w-0 break-all font-mono text-text">
            {request.exePath}
          </dd>
        </div>
        {request.gitRemote && (
          <div className="flex gap-2">
            <dt className="w-12 shrink-0 text-text-muted">Git</dt>
            <dd className="min-w-0 break-all font-mono text-text">
              {request.gitRemote}
            </dd>
          </div>
        )}
        {displayArgs && (
          <div className="flex gap-2">
            <dt className="w-12 shrink-0 text-text-muted">Args</dt>
            <dd className="min-w-0 break-all font-mono text-text-muted">
              {displayArgs}
            </dd>
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
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => handle(false)}
          >
            Deny
          </Button>
          <Button
            variant="primary"
            size="sm"
            disabled={busy}
            onClick={() => handle(true)}
          >
            Allow
          </Button>
        </div>
      </div>
    </div>
  );
}

function GrantCard({
  grant,
  onRevoke,
}: {
  grant: GrantRow;
  onRevoke: (id: string) => void;
}) {
  const [showDetails, setShowDetails] = useState(false);
  const displayArgs = grant.runArgs ? stripSensitiveArgs(grant.runArgs) : "";

  return (
    <>
      <div className="flex items-center justify-between rounded-lg border border-border bg-surface p-4">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span
              className={`inline-block size-2 rounded-full ${grant.isActive ? "bg-green-500" : "bg-neutral-400"}`}
            />
            <span className="text-sm font-medium text-text">
              {grant.bucketName}
            </span>
            {grant.clientLabel && (
              <span className="truncate text-xs text-text-muted">
                — {grant.clientLabel}
              </span>
            )}
          </div>
          <div className="mt-1 flex flex-wrap gap-3 text-[11px] text-text-muted">
            <span>Granted {timeAgo(grant.grantedAt)}</span>
            {grant.isActive && (
              <span className="text-green-600">
                Expires in {expiresIn(grant.expiresAt)}
              </span>
            )}
            {!grant.isActive && <span>Expired</span>}
            {grant.lastSeenAt && <span>Last used {timeAgo(grant.lastSeenAt)}</span>}
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
          <Button variant="danger" size="sm" onClick={() => onRevoke(grant.id)}>
            Revoke
          </Button>
        </div>
      </div>

      {showDetails && (
        <GrantDetailsModal grant={grant} displayArgs={displayArgs} onClose={() => setShowDetails(false)} />
      )}
    </>
  );
}

function GrantDetailsModal({
  grant,
  displayArgs,
  onClose,
}: {
  grant: GrantRow;
  displayArgs: string;
  onClose: () => void;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="w-full max-w-md rounded-lg border border-border bg-surface p-5 shadow-xl">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-text">Grant Details</h3>
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
