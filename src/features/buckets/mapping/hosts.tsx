import { useEffect, useState } from "react";
import { Plus, X } from "lucide-react";

import { cn } from "@/core/cn";
import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";

/** Stored in allowedHosts when all domains are permitted for proxy rewrite. */
export const ALLOW_ALL_DOMAINS = "*";

export function isAllowAllDomains(hosts: string[]): boolean {
  return hosts.length === 1 && hosts[0] === ALLOW_ALL_DOMAINS;
}

interface MappingAllowedHostsProps {
  hosts: string[];
  onChange: (hosts: string[]) => void;
  disabled?: boolean;
}

export function MappingAllowedHosts({ hosts, onChange, disabled }: MappingAllowedHostsProps) {
  const [hostDraft, setHostDraft] = useState("");
  const allowAll = isAllowAllDomains(hosts);
  const [restrictedHosts, setRestrictedHosts] = useState<string[]>(() =>
    allowAll ? [] : hosts.filter((h) => h !== ALLOW_ALL_DOMAINS),
  );

  useEffect(() => {
    if (!allowAll) {
      setRestrictedHosts(hosts.filter((h) => h !== ALLOW_ALL_DOMAINS));
    }
  }, [hosts, allowAll]);

  function setMode(all: boolean) {
    if (disabled) return;
    if (all) {
      if (!allowAll) {
        setRestrictedHosts(hosts.filter((h) => h !== ALLOW_ALL_DOMAINS));
      }
      onChange([ALLOW_ALL_DOMAINS]);
    } else {
      onChange(restrictedHosts);
    }
  }

  function addHost() {
    const h = hostDraft.trim();
    if (!h || h === ALLOW_ALL_DOMAINS || restrictedHosts.includes(h)) return;
    const next = [...restrictedHosts, h];
    setRestrictedHosts(next);
    onChange(next);
    setHostDraft("");
  }

  function removeHost(host: string) {
    const next = restrictedHosts.filter((x) => x !== host);
    setRestrictedHosts(next);
    onChange(next);
  }

  return (
    <div>
      <p className="text-xs font-medium text-text-muted">Allowed domains</p>

      <div
        className="mt-2 inline-flex rounded-md border border-border p-0.5"
        role="group"
        aria-label="Domain access mode"
      >
        <button
          type="button"
          disabled={disabled}
          aria-pressed={allowAll ? "true" : "false"}
          onClick={() => setMode(true)}
          className={cn(
            "rounded px-3 py-1.5 text-xs font-medium transition-colors",
            allowAll
              ? "bg-accent text-white"
              : "text-text-muted hover:text-text",
            disabled && "cursor-not-allowed opacity-50",
          )}
        >
          Allow all
        </button>
        <button
          type="button"
          disabled={disabled}
          aria-pressed={!allowAll ? "true" : "false"}
          onClick={() => setMode(false)}
          className={cn(
            "rounded px-3 py-1.5 text-xs font-medium transition-colors",
            !allowAll
              ? "bg-accent text-white"
              : "text-text-muted hover:text-text",
            disabled && "cursor-not-allowed opacity-50",
          )}
        >
          Restrict
        </button>
      </div>

      {allowAll ? (
        <p className="mt-2 text-xs text-text-muted">All domains are allowed for this key.</p>
      ) : (
        <>
          <p className="mt-2 text-xs text-text-muted">
            Only these domains are forwarded for this key (e.g. openai.com allows api.openai.com).
            Empty list blocks all.
          </p>
          <div className="mt-2 flex flex-wrap gap-1.5">
            {restrictedHosts.map((host) => (
              <span
                key={host}
                className="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2 py-0.5 text-xs"
              >
                {host}
                <button
                  type="button"
                  className="text-text-muted hover:text-text"
                  onClick={() => removeHost(host)}
                  disabled={disabled}
                  aria-label={`Remove ${host}`}
                >
                  <X className="size-3" />
                </button>
              </span>
            ))}
          </div>
          <div className="mt-2 flex gap-2">
            <ArgusInput
              placeholder="openai.com"
              value={hostDraft}
              onChange={(e) => setHostDraft(e.target.value)}
              disabled={disabled}
              onKeyDown={(e) => {
                if (e.key === "Enter") addHost();
              }}
            />
            <Button type="button" variant="secondary" size="sm" onClick={addHost} disabled={disabled}>
              <Plus className="size-4" />
              Add
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
