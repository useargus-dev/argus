import { useState } from "react";
import { Plus, X } from "lucide-react";

import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";

interface MappingAllowedHostsProps {
  hosts: string[];
  onChange: (hosts: string[]) => void;
  disabled?: boolean;
}

export function MappingAllowedHosts({ hosts, onChange, disabled }: MappingAllowedHostsProps) {
  const [hostDraft, setHostDraft] = useState("");

  function addHost() {
    const h = hostDraft.trim();
    if (!h || hosts.includes(h)) return;
    onChange([...hosts, h]);
    setHostDraft("");
  }

  function removeHost(host: string) {
    onChange(hosts.filter((x) => x !== host));
  }

  return (
    <div>
      <p className="text-xs font-medium text-text-muted">Allowed hosts</p>
      <p className="mt-0.5 text-xs text-text-muted">
        Only these domains are forwarded for this key (e.g. openai.com allows api.openai.com). Empty list blocks all.
      </p>
      <div className="mt-2 flex flex-wrap gap-1.5">
        {hosts.map((host) => (
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
    </div>
  );
}
