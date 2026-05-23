import { Outlet } from "react-router-dom";

import { Sidebar } from "./sidebar";

export function AppShell() {
  return (
    <div className="flex min-h-dvh bg-bg">
      <Sidebar />
      <main className="flex-1 overflow-auto p-6">
        <Outlet />
      </main>
    </div>
  );
}
