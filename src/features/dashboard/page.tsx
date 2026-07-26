import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  Activity,
  ArrowRight,
  KeyRound,
  Lock,
  Package,
  Plus,
  Sparkles,
  TriangleAlert,
} from "lucide-react";

import { bridge } from "@/core/bridge";
import { useTauriEvent } from "@/shared/hooks/event";
import type { SecretMeta } from "@/shared/types/secret";
import type { BucketMeta } from "@/shared/types/bucket";
import { fmtDays, fmtRel } from "@/shared/utils/time";

export function DashboardPage() {
  const [secrets, setSecrets] = useState<SecretMeta[]>([]);
  const [buckets, setBuckets] = useState<BucketMeta[]>([]);
  const [pendingCount, setPendingCount] = useState(0);
  const [loadError, setLoadError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [s, b, p] = await Promise.all([
        bridge.searchSecrets(),
        bridge.listBuckets(),
        bridge.pendingCount(),
      ]);
      setSecrets(s);
      setBuckets(b);
      setPendingCount(p);
      setLoadError(null);
    } catch {
      setLoadError("Could not load dashboard (locked or signed out).");
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useTauriEvent("client-access-requested", () => refresh());
  useTauriEvent("client-access-resolved", () => refresh());
  useTauriEvent("grants-changed", () => refresh());

  const activeSecrets = secrets.filter((s) => !s.isArchived);
  const orgs = new Set(activeSecrets.map((s) => s.organization).filter(Boolean));
  const activeBuckets = buckets.filter((b) => b.isActive);
  const totalActiveClients = buckets.reduce((sum, b) => sum + b.activeGrantCount, 0);

  const expiring = activeSecrets
    .filter((s) => s.expiresAt && fmtDays(s.expiresAt) <= 30)
    .sort((a, b) => fmtDays(a.expiresAt!) - fmtDays(b.expiresAt!));

  return (
    <div className="mx-auto max-w-[1400px] px-8 py-6">
      <div className="mb-6">
        <h1 className="text-2xl font-semibold tracking-tight text-text">
          Dashboard
        </h1>
        <p className="mt-1 text-sm text-text-muted">
          Privacy-only · all secrets stay on this device
        </p>
        {loadError && (
          <p className="mt-2 text-sm text-danger">{loadError}</p>
        )}
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
        {/* Vault */}
        <div className="flex flex-col rounded-xl border border-border bg-surface p-5">
          <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Vault
          </h3>
          <div className="flex items-baseline gap-2">
            <span className="text-4xl font-semibold tabular-nums">
              {activeSecrets.length}
            </span>
            <span className="text-xs text-text-muted">secrets</span>
          </div>
          <div className="mt-3 flex flex-wrap gap-1.5">
            <span className="inline-flex items-center gap-1 rounded-md border border-accent/30 bg-accent/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-accent">
              <KeyRound className="size-3" aria-hidden />
              {activeSecrets.length} active
            </span>
            {orgs.size > 0 && (
              <span className="inline-flex items-center gap-1 rounded-md border border-border bg-surface-raised px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-text-muted">
                {orgs.size} orgs
              </span>
            )}
          </div>
          <Link
            to="/vault"
            className="mt-auto inline-flex items-center gap-1 pt-3 text-xs text-accent hover:text-accent-hover"
          >
            Open vault <ArrowRight className="size-3" aria-hidden />
          </Link>
        </div>

        {/* App Buckets */}
        <div className="flex flex-col rounded-xl border border-border bg-surface p-5">
          <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            App buckets
          </h3>
          <div className="flex items-baseline gap-2">
            <span className="text-4xl font-semibold tabular-nums">
              {buckets.length}
            </span>
            <span className="text-xs text-text-muted">buckets</span>
          </div>
          <div className="mt-3 flex flex-wrap gap-1.5">
            {activeBuckets.length > 0 && (
              <span className="inline-flex items-center gap-1 rounded-md border border-success/30 bg-success/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-success">
                <Package className="size-3" aria-hidden />
                {totalActiveClients} active client{totalActiveClients !== 1 ? "s" : ""}
              </span>
            )}
          </div>
          <Link
            to="/buckets"
            className="mt-auto inline-flex items-center gap-1 pt-3 text-xs text-accent hover:text-accent-hover"
          >
            Manage buckets <ArrowRight className="size-3" aria-hidden />
          </Link>
        </div>

        {/* Expiring soon */}
        <div className="flex flex-col rounded-xl border border-border bg-surface p-5 md:col-span-2">
          <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Expiring soon
          </h3>
          <div className="min-h-0 flex-1">
            {expiring.length === 0 ? (
              <p className="text-sm text-text-muted">
                No secrets expiring in the next 30 days.
              </p>
            ) : (
              <ul className="divide-y divide-border">
                {expiring.slice(0, 4).map((s) => {
                  const days = fmtDays(s.expiresAt!);
                  const urgent = days <= 7;
                  return (
                    <li
                      key={s.id}
                      className="flex items-center gap-3 py-2.5 first:pt-0 last:pb-0"
                    >
                      <TriangleAlert
                        className={
                          urgent
                            ? "size-4 text-danger"
                            : "size-4 text-warning"
                        }
                        aria-hidden
                      />
                      <div className="min-w-0 flex-1">
                        <div className="truncate text-sm font-medium">
                          {s.name}
                        </div>
                        <div className="text-xs text-text-muted">
                          {s.organization ?? "—"} · {s.environment ?? "—"}
                        </div>
                      </div>
                      <span
                        className={
                          urgent
                            ? "inline-flex items-center gap-1 rounded-md border border-danger/30 bg-danger/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-danger"
                            : "inline-flex items-center gap-1 rounded-md border border-warning/30 bg-warning/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-warning"
                        }
                      >
                        {days}d
                      </span>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </div>

        {/* Pending approvals */}
        <div className="flex flex-col rounded-xl border border-border bg-surface p-5">
          <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Pending approvals
          </h3>
          <div className="flex items-baseline gap-2">
            <span
              className={`text-4xl font-semibold tabular-nums ${pendingCount > 0 ? "text-accent" : ""}`}
            >
              {pendingCount}
            </span>
            <span className="text-xs text-text-muted">awaiting</span>
          </div>
          <p className="mt-3 text-xs text-text-muted">
            {pendingCount > 0
              ? "Decide via the Approvals page or system tray."
              : "No pending access requests."}
          </p>
          {pendingCount > 0 && (
            <Link
              to="/approvals"
              className="mt-auto inline-flex items-center gap-1 pt-3 text-xs text-accent hover:text-accent-hover"
            >
              View approvals <ArrowRight className="size-3" aria-hidden />
            </Link>
          )}
        </div>

        {/* Quick actions */}
        <div className="flex flex-col rounded-xl border border-border bg-surface p-5">
          <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Quick actions
          </h3>
          <div className="space-y-2">
            <Link
              to="/vault"
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-text-muted transition-colors hover:bg-surface-raised hover:text-text"
            >
              <Plus className="size-4" aria-hidden />
              Add secret
            </Link>
            <Link
              to="/buckets"
              className="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm text-text-muted transition-colors hover:bg-surface-raised hover:text-text"
            >
              <Package className="size-4" aria-hidden />
              New bucket
            </Link>
            <button
              type="button"
              onClick={() => bridge.lockApp().catch(() => {})}
              className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-text-muted transition-colors hover:bg-surface-raised hover:text-text"
            >
              <Lock className="size-4" aria-hidden />
              Lock App
            </button>
          </div>
        </div>

        {/* Recent activity */}
        <div className="flex flex-col rounded-xl border border-border bg-surface p-5 md:col-span-2">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-[11px] font-semibold uppercase tracking-wider text-text-muted">
              Recent activity
            </h3>
            <Activity className="size-3.5 text-text-muted" aria-hidden />
          </div>
          <div className="min-h-0 flex-1">
            <RecentActivity secrets={secrets} buckets={buckets} />
          </div>
        </div>

        {/* Tip */}
        <div className="flex flex-col rounded-xl border border-accent/20 bg-gradient-to-br from-surface to-surface-raised p-5 md:col-span-4">
          <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Tip
          </h3>
          <div className="flex items-center gap-3">
            <Sparkles className="size-5 shrink-0 text-accent" aria-hidden />
            <p className="text-sm text-text-muted">
              Add{" "}
              <code className="rounded bg-surface-raised px-1.5 py-0.5 font-mono text-xs text-accent">
                ARGUS_BUCKET_ID
              </code>{" "}
              and{" "}
              <code className="rounded bg-surface-raised px-1.5 py-0.5 font-mono text-xs text-accent">
                ARGUS_BUCKET_TOKEN
              </code>{" "}
              to your project's <span className="font-mono">.env</span> to inject
              secrets into your app without ever writing them to disk.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

function RecentActivity({
  secrets,
  buckets,
}: {
  secrets: SecretMeta[];
  buckets: BucketMeta[];
}) {
  type ActivityItem = { label: string; detail: string; time: string; at: number };
  const items: ActivityItem[] = [];

  for (const s of secrets.slice(0, 5)) {
    items.push({
      label: "SECRET_CREATED",
      detail: s.name,
      time: fmtRel(s.createdAt),
      at: new Date(s.createdAt).getTime(),
    });
    if (s.updatedAt !== s.createdAt) {
      items.push({
        label: "SECRET_UPDATED",
        detail: s.name,
        time: fmtRel(s.updatedAt),
        at: new Date(s.updatedAt).getTime(),
      });
    }
  }
  for (const b of buckets.slice(0, 3)) {
    items.push({
      label: "BUCKET_CREATED",
      detail: b.name,
      time: fmtRel(b.createdAt),
      at: new Date(b.createdAt).getTime(),
    });
  }

  items.sort((a, b) => b.at - a.at);
  const display = items.slice(0, 5);

  if (display.length === 0) {
    return <p className="text-sm text-text-muted">No activity yet.</p>;
  }

  return (
    <ul className="space-y-2.5">
      {display.map((item, i) => (
        <li key={i} className="flex items-center gap-3 text-sm">
          <span className="size-1.5 shrink-0 rounded-full bg-accent" />
          <span className="w-36 shrink-0 font-mono text-[11px] uppercase tracking-wider text-text-muted">
            {item.label}
          </span>
          <span className="flex-1 truncate">{item.detail}</span>
          <span className="text-xs text-text-muted">{item.time}</span>
        </li>
      ))}
    </ul>
  );
}
