import { useCallback, useEffect, useState } from "react";

import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import { useTauriEvent } from "@/shared/hooks/event";
import type { ClientAccessRequest, GrantRow } from "@/shared/types/client";

import { GrantCard } from "./grant";
import { PendingCard } from "./pending";

export function ApprovalsPage() {
  const [grants, setGrants] = useState<GrantRow[]>([]);
  const [pending, setPending] = useState<ClientAccessRequest[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const [grantList, pendingList] = await Promise.all([
        bridge.listGrants(),
        bridge.listPending(),
      ]);
      setGrants(grantList.filter((g) => g.isActive));
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
        await bridge.respondAccess({ requestId, accept, ttlMinutes });
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

  return (
    <div className="mx-auto max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-text">Approvals</h1>
        <p className="mt-1 text-sm text-text-muted">
          Pending requests and active access grants for your buckets.
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

      {grants.length > 0 && (
        <section>
          <h2 className="mb-3 text-sm font-semibold uppercase tracking-wider text-text-muted">
            Active ({grants.length})
          </h2>
          <div className="space-y-2">
            {grants.map((g) => (
              <GrantCard key={g.id} grant={g} onRevoke={revoke} />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
