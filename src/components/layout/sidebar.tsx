import { NavLink } from "react-router-dom";
import { Eye, LayoutDashboard, Settings } from "lucide-react";

import { cn } from "../../lib/cn";
import { useAuthStore } from "../../state/auth-store";

const links = [
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const profile = useAuthStore((s) => s.profile);

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-border bg-surface">
      <div className="flex items-center gap-2 border-b border-border px-4 py-4">
        <Eye className="text-accent" size={22} />
        <span className="font-semibold text-text">Argus</span>
      </div>
      <nav className="flex flex-1 flex-col gap-1 p-3">
        {links.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors",
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
        <div className="border-t border-border p-4">
          <p className="truncate text-sm font-medium text-text">
            {profile.username}
          </p>
          <p className="truncate text-xs text-text-muted">{profile.email}</p>
        </div>
      )}
    </aside>
  );
}
