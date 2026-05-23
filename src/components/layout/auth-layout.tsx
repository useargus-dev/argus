import type { ReactNode } from "react";
import { Eye } from "lucide-react";

import { Card } from "../ui/card";

export function AuthLayout({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex min-h-dvh items-center justify-center bg-bg p-4">
      <div className="w-full max-w-md space-y-6">
        <div className="flex flex-col items-center gap-2 text-center">
          <div className="flex h-12 w-12 items-center justify-center rounded-full bg-surface-raised text-accent">
            <Eye size={28} />
          </div>
          <h1 className="text-2xl font-semibold text-text">{title}</h1>
          {subtitle && (
            <p className="text-sm text-text-muted">{subtitle}</p>
          )}
        </div>
        <Card>{children}</Card>
      </div>
    </div>
  );
}
