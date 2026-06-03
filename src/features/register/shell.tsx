import type { ReactNode } from "react";

import { AppBrand } from "@/shared/ui/brand";
import { RegisterProgress } from "./progress";

interface RegisterShellProps {
  step: 1 | 2 | 3;
  title: string;
  subtitle: string;
  children: ReactNode;
}

export function RegisterShell({
  step,
  title,
  subtitle,
  children,
}: RegisterShellProps) {
  return (
    <div className="grid min-h-screen place-items-center bg-bg px-4 pb-20">
      <div className="w-full max-w-md">
        <AppBrand />
        <div className="argus-card p-6">
          <RegisterProgress step={step} />
          {title ? (
            <h1 className="mb-1 text-base font-semibold text-text">{title}</h1>
          ) : null}
          {subtitle ? (
            <p className="mb-5 text-xs text-text-muted">{subtitle}</p>
          ) : title ? (
            <div className="mb-5" />
          ) : null}
          {children}
        </div>
      </div>
    </div>
  );
}
