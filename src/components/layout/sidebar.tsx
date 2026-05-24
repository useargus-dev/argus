import { NavLink } from "react-router-dom";
import { Eclipse, LayoutDashboard, Lock, Package, Settings } from "lucide-react";

import { cn } from "../../lib/cn";
import { useAuthStore } from "../../state/auth-store";

const links = [
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/vault", label: "Vault", icon: Lock },
  { to: "/buckets", label: "Buckets", icon: Package },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const profile = useAuthStore((s) => s.profile);

  return (
    <aside className="z-10 flex h-dvh w-60 shrink-0 flex-col overflow-hidden border-r border-border bg-surface">
      <div className="flex shrink-0 items-center gap-2 border-b border-border px-4 py-4">
        <Eclipse className="text-signal" size={22} />
        <span className="font-semibold text-text">Argus</span>
      </div>
      <nav className="flex min-h-0 flex-1 flex-col gap-1 overflow-hidden p-3">
        {links.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              cn(
                "flex shrink-0 items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
                isActive
                  ? "bg-surface-raised text-text"
                  : "text-text-muted hover:bg-surface-raised hover:text-text",
              )
            }
          >
            <Icon size={18} />
            {label}
          </NavLink>
        ))}
      </nav>
      {profile && (
        <div className="shrink-0 border-t border-border p-4">
          <p className="truncate text-sm font-medium text-text">
            {profile.username}
          </p>
          <p className="truncate text-xs text-text-muted">{profile.email}</p>
        </div>
      )}
    </aside>
  );
}
