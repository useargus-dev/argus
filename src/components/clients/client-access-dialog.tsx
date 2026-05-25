import { useCallback, useEffect, useState } from "react";

import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import type { ClientAccessRequest } from "../../types/client";
import { Button } from "../ui/button";

const TTL_OPTIONS = [15, 60, 180, 480] as const;

interface ClientAccessDialogProps {
  request: ClientAccessRequest | null;
  onClose: () => void;
}

/**
 * @deprecated The requests popup window is now used instead of this modal.
 */
export function ClientAccessDialog({ request, onClose }: ClientAccessDialogProps) {
  const [ttl, setTtl] = useState(60);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (request) setTtl(request.accessTtlMinutes || 60);
  }, [request?.requestId]);

  const respond = useCallback(
    async (accept: boolean) => {
      if (!request) return;
      setBusy(true);
      try {
        await bridge.respondToClientAccess({
          requestId: request.requestId,
          accept,
          ttlMinutes: accept ? ttl : undefined,
        });
        toast.success(accept ? "Access granted" : "Access denied");
        onClose();
      } catch (e) {
        toast.fromError(e, accept ? "Could not grant access" : "Could not deny access");
      } finally {
        setBusy(false);
      }
    },
    [request, ttl, onClose],
  );

  if (!request) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"
      role="presentation"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="client-access-title"
        className="w-full max-w-lg rounded-xl border border-border bg-surface p-6 shadow-xl"
      >
        <h2 id="client-access-title" className="text-lg font-semibold text-text">
          Application access request
        </h2>
        <p className="mt-2 text-sm text-text-muted">
          An app wants to load environment variables from bucket{" "}
          <span className="font-medium text-text">{request.bucketName}</span>.
        </p>

        <dl className="mt-4 space-y-2 rounded-md border border-border bg-surface-raised/60 p-3 text-xs">
          <div>
            <dt className="text-text-muted">Working directory</dt>
            <dd className="mt-0.5 break-all font-mono text-text">
              {request.cwd}
              {!request.cwdVerified && (
                <span className="ml-1 text-yellow-500">(unverified)</span>
              )}
            </dd>
          </div>
          <div>
            <dt className="text-text-muted">Executable</dt>
            <dd className="mt-0.5 break-all font-mono text-text">
              {request.processName} (pid {request.pid})
            </dd>
            <dd className="mt-0.5 break-all font-mono text-text-muted">
              {request.exePath}
            </dd>
          </div>
          {request.gitRemote && (
            <div>
              <dt className="text-text-muted">Git remote</dt>
              <dd className="mt-0.5 break-all font-mono text-text">
                {request.gitRemote}
              </dd>
            </div>
          )}
        </dl>

        <div className="mt-4">
          <span className="text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Grant for
          </span>
          <div className="mt-2 flex flex-wrap gap-2">
            {TTL_OPTIONS.map((m) => (
              <button
                key={m}
                type="button"
                disabled={busy}
                onClick={() => setTtl(m)}
                className={
                  ttl === m
                    ? "rounded-md border border-accent bg-accent/10 px-3 py-1.5 text-xs font-medium text-accent"
                    : "rounded-md border border-border px-3 py-1.5 text-xs text-text-muted hover:bg-surface-raised"
                }
              >
                {m < 60 ? `${m}m` : `${m / 60}h`}
              </button>
            ))}
          </div>
        </div>

        <div className="mt-6 flex justify-end gap-2">
          <Button
            type="button"
            variant="ghost"
            disabled={busy}
            onClick={() => respond(false)}
          >
            Deny
          </Button>
          <Button
            type="button"
            variant="primary"
            disabled={busy}
            onClick={() => respond(true)}
          >
            Allow access
          </Button>
        </div>
      </div>
    </div>
  );
}
