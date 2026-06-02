import type { ReactNode } from "react";

interface SettingsRowProps {
  label: string;
  children: ReactNode;
  description?: string;
}

export function SettingsRow({ label, children, description }: SettingsRowProps) {
  return (
    <div className="min-h-9">
      <div className="flex items-center justify-between gap-4">
        <span className="text-sm text-text-muted">{label}</span>
        <div className="flex min-w-0 max-w-[min(100%,16rem)] flex-1 items-center justify-end gap-2">
          {children}
        </div>
      </div>
      {description && (
        <p className="mt-1 text-end text-xs text-text-muted">{description}</p>
      )}
    </div>
  );
}
