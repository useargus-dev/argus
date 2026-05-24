import { useEffect, useRef, useState } from "react";
import { Search } from "lucide-react";

import { secretTypeLabel } from "../../lib/secret-utils";
import { cn } from "../../lib/cn";
import type { SecretMeta } from "../../types/secret";

interface SecretPickerProps {
  secrets: SecretMeta[];
  value: string;
  onChange: (secretId: string) => void;
  disabled?: boolean;
  placeholder?: string;
}

export function SecretPicker({
  secrets,
  value,
  onChange,
  disabled,
  placeholder = "Search vault secrets…",
}: SecretPickerProps) {
  const rootRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");

  const selected = secrets.find((s) => s.id === value);

  useEffect(() => {
    if (selected) {
      setQuery(selected.name);
    } else if (!value) {
      setQuery("");
    }
  }, [selected, value]);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const q = query.trim().toLowerCase();
  const filtered = secrets.filter((s) => {
    if (!q) return true;
    const haystack = [s.name, s.organization ?? "", s.environment ?? "", s.secretType]
      .join(" ")
      .toLowerCase();
    return haystack.includes(q);
  });

  function pick(secret: SecretMeta) {
    onChange(secret.id);
    setQuery(secret.name);
    setOpen(false);
  }

  return (
    <div ref={rootRef} className="relative min-w-0 flex-1">
      <div className="relative">
        <Search
          className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-text-muted"
          aria-hidden
        />
        <input
          type="search"
          value={query}
          disabled={disabled}
          placeholder={placeholder}
          onFocus={() => setOpen(true)}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
            if (!e.target.value.trim()) onChange("");
          }}
          className="h-9 w-full rounded-md border border-border bg-surface pl-9 pr-3 text-sm placeholder:text-text-muted focus:border-accent focus:outline-none disabled:opacity-50"
        />
      </div>
      {open && !disabled && (
        <ul
          className={cn(
            "absolute z-20 mt-1 max-h-52 w-full overflow-auto rounded-md border border-border bg-surface py-1 shadow-lg",
            filtered.length === 0 && "px-3 py-2 text-sm text-text-muted",
          )}
          role="listbox"
        >
          {filtered.length === 0 ? (
            <li>No secrets match</li>
          ) : (
            filtered.map((s) => (
              <li key={s.id} role="option">
                <button
                  type="button"
                  className={cn(
                    "flex w-full flex-col items-start px-3 py-2 text-left text-sm hover:bg-surface-raised",
                    s.id === value && "bg-surface-raised",
                  )}
                  onClick={() => pick(s)}
                >
                  <span className="font-medium">{s.name}</span>
                  <span className="text-xs text-text-muted">
                    {secretTypeLabel(s.secretType)}
                    {s.environment ? ` · ${s.environment}` : ""}
                  </span>
                </button>
              </li>
            ))
          )}
        </ul>
      )}
    </div>
  );
}
