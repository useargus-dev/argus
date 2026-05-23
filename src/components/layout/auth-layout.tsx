import type { ReactNode } from "react";

import { RegisterBrand } from "../register/brand";

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
    <div className="grid min-h-screen place-items-center bg-bg px-4 pb-20">
      <div className="w-full max-w-md">
        <RegisterBrand />
        <div className="argus-card p-6">
          <h1 className="mb-1 text-base font-semibold text-text">{title}</h1>
          {subtitle && (
            <p className="mb-5 text-xs text-text-muted">{subtitle}</p>
          )}
          {children}
        </div>
      </div>
    </div>
  );
}
