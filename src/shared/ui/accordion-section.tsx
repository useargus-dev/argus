import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

import { cn } from "@/core/cn";

interface AccordionSectionProps {
  title: string;
  description?: string;
  /** Rendered on the title row (e.g. toggle). Clicks do not expand/collapse. */
  headerAction?: ReactNode;
  defaultOpen?: boolean;
  children: ReactNode;
  className?: string;
}

export function AccordionSection({
  title,
  description,
  headerAction,
  defaultOpen = false,
  children,
  className,
}: AccordionSectionProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className={cn("overflow-hidden rounded-xl border border-border bg-surface", className)}>
      <div className="flex w-full items-center gap-3 px-5 py-4">
        <button
          type="button"
          className="flex min-w-0 flex-1 items-start gap-3 text-left"
          onClick={() => setOpen((v) => !v)}
          aria-expanded={open}
        >
          <ChevronDown
            className={cn(
              "mt-0.5 size-4 shrink-0 text-text-muted transition-transform",
              open && "rotate-180",
            )}
            aria-hidden
          />
          <h2 className="min-w-0 text-sm font-semibold">{title}</h2>
        </button>
        {headerAction != null && (
          <div
            className="flex shrink-0 items-center gap-2"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => e.stopPropagation()}
          >
            {headerAction}
          </div>
        )}
      </div>
      {open && (
        <div className="border-t border-border px-5 pb-5 pt-4">
          {description && (
            <p className="mb-4 text-xs text-text-muted">{description}</p>
          )}
          {children}
        </div>
      )}
    </div>
  );
}
