import { Outlet } from "react-router-dom";

import { Sidebar } from "./sidebar";

export function AppShell() {
  return (
    <div className="flex h-dvh overflow-hidden bg-bg">
      <Sidebar />
      <main className="min-h-0 min-w-0 flex-1 overflow-y-auto p-6 pb-20">
        <Outlet />
      </main>
    </div>
  );
}
