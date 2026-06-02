import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

interface SettingsSectionProps {
  title: string;
  icon: LucideIcon;
  children: ReactNode;
  variant?: "default" | "danger";
}

export function SettingsSection({
  title,
  icon: Icon,
  children,
  variant = "default",
}: SettingsSectionProps) {
  const isDanger = variant === "danger";

  return (
    <section
      className={
        isDanger
          ? "rounded-xl border border-danger/30 bg-danger/5 p-5"
          : "rounded-xl border border-border bg-surface p-5"
      }
    >
      <h2
        className={
          isDanger
            ? "mb-4 flex items-center gap-2 text-sm font-semibold text-danger"
            : "mb-4 flex items-center gap-2 text-sm font-semibold text-text"
        }
      >
        <span className={isDanger ? "text-danger" : "text-signal"}>
          <Icon className="size-4" aria-hidden />
        </span>
        {title}
      </h2>
      <div className="space-y-3">{children}</div>
    </section>
  );
}
