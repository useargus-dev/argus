import { useEffect, useState } from "react";
import { Copy, Eye, EyeOff } from "lucide-react";

import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import { cn } from "../../lib/cn";

interface BucketEnvCredentialsProps {
  bucketId: string;
  cachedToken?: string | null;
  onTokenCached?: (token: string) => void;
  className?: string;
}

/** Matches `TOKEN_LEN` in `src-tauri/src/db/buckets.rs`. */
const BUCKET_TOKEN_MASK_LEN = 32;

function maskBucketId(value: string): string {
  return `${value.slice(0, 8)}${"•".repeat(12)}`;
}

const maskedToken = "•".repeat(BUCKET_TOKEN_MASK_LEN);

export function BucketEnvCredentials({
  bucketId,
  cachedToken,
  onTokenCached,
  className,
}: BucketEnvCredentialsProps) {
  const [revealed, setRevealed] = useState(false);
  const [token, setToken] = useState<string | null>(cachedToken ?? null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setRevealed(false);
  }, [bucketId]);

  useEffect(() => {
    setToken(cachedToken ?? null);
  }, [cachedToken]);

  async function loadToken() {
    if (token) return token;
    setLoading(true);
    try {
      const t = await bridge.getBucketToken(bucketId);
      setToken(t);
      onTokenCached?.(t);
      return t;
    } catch (e) {
      toast.fromError(e, "Could not load bucket credentials");
      return null;
    } finally {
      setLoading(false);
    }
  }

  async function handleReveal() {
    if (revealed) {
      setRevealed(false);
      return;
    }
    const t = token ?? (await loadToken());
    if (t) setRevealed(true);
  }

  async function handleCopy() {
    const t = token ?? (await loadToken());
    if (!t) return;
    const text = `ARGUS_BUCKET_ID=${bucketId}\nARGUS_BUCKET_TOKEN=${t}`;
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied to clipboard");
    } catch {
      toast.error("Could not copy to clipboard");
    }
  }

  const bucketIdLine = revealed
    ? `ARGUS_BUCKET_ID=${bucketId}`
    : `ARGUS_BUCKET_ID=${maskBucketId(bucketId)}`;

  const tokenLine =
    revealed && token
      ? `ARGUS_BUCKET_TOKEN=${token}`
      : `ARGUS_BUCKET_TOKEN=${maskedToken}`;

  return (
    <div className={className}>
      <div className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-text-muted">
        Project .env
      </div>
      <div
        className={cn(
          "flex gap-2 rounded-md border border-border bg-surface-raised/60 px-3 py-2",
        )}
      >
        <div className="min-w-0 flex-1 space-y-1">
          <code className="block truncate font-mono text-xs text-text">
            {bucketIdLine}
          </code>
          <code className="block truncate font-mono text-xs text-text">
            {tokenLine}
          </code>
        </div>
        <div className="flex shrink-0 items-center gap-0.5 self-center">
          <button
            type="button"
            aria-label={revealed ? "Hide credentials" : "Reveal credentials"}
            title={revealed ? "Hide" : "Reveal"}
            disabled={loading}
            onClick={handleReveal}
            className="grid size-7 place-items-center rounded text-text-muted transition-colors hover:bg-surface-raised hover:text-text"
          >
            {revealed ? (
              <EyeOff className="size-3.5" aria-hidden />
            ) : (
              <Eye className="size-3.5" aria-hidden />
            )}
          </button>
          <button
            type="button"
            aria-label="Copy credentials"
            title="Copy"
            disabled={loading}
            onClick={handleCopy}
            className="grid size-7 place-items-center rounded text-text-muted transition-colors hover:bg-accent/10 hover:text-accent"
          >
            <Copy className="size-3.5" aria-hidden />
          </button>
        </div>
      </div>
    </div>
  );
}
