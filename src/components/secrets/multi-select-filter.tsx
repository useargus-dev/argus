import { ChevronDown } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { cn } from "../../lib/cn";

interface MultiSelectOption {
  value: string;
  label: string;
}

interface MultiSelectFilterProps {
  label: string;
  options: MultiSelectOption[];
  selected: string[];
  onChange: (selected: string[]) => void;
  className?: string;
}

export function MultiSelectFilter({
  label,
  options,
  selected,
  onChange,
  className,
}: MultiSelectFilterProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function onDoc(e: MouseEvent) {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onDoc);
    return () => document.removeEventListener("mousedown", onDoc);
  }, [open]);

  const summary =
    selected.length === 0
      ? `All ${label.toLowerCase()}`
      : selected.length === 1
        ? (options.find((o) => o.value === selected[0])?.label ?? selected[0])
        : `${selected.length} selected`;

  function toggle(value: string) {
    if (selected.includes(value)) {
      onChange(selected.filter((v) => v !== value));
    } else {
      onChange([...selected, value]);
    }
  }

  return (
    <div ref={rootRef} className={cn("relative", className)}>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="inline-flex h-9 min-w-[9rem] items-center justify-between gap-2 rounded-md border border-border bg-surface px-3 text-sm text-text hover:border-accent focus:border-accent focus:outline-none"
      >
        <span className="truncate">{summary}</span>
        <ChevronDown className="size-4 shrink-0 text-text-muted" aria-hidden />
      </button>
      {open && (
        <div className="absolute right-0 z-20 mt-1 max-h-60 min-w-full overflow-auto rounded-md border border-border bg-surface py-1 shadow-lg">
          {options.length === 0 ? (
            <p className="px-3 py-2 text-xs text-text-muted">No options</p>
          ) : (
            options.map((opt) => (
              <label
                key={opt.value}
                className="flex cursor-pointer items-center gap-2 px-3 py-1.5 text-sm hover:bg-surface-raised"
              >
                <input
                  type="checkbox"
                  checked={selected.includes(opt.value)}
                  onChange={() => toggle(opt.value)}
                  className="size-3.5 rounded border-border accent-accent"
                />
                <span className="truncate">{opt.label}</span>
              </label>
            ))
          )}
        </div>
      )}
    </div>
  );
}
