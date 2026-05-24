import { useEffect } from "react";
import { Outlet } from "react-router-dom";

import { AppLockModal } from "../app/app-lock-modal";
import { bridge } from "../../lib/tauri-bridge";
import { useAuthStore } from "../../state/auth-store";
import { Sidebar } from "./sidebar";

function SessionWatchdog() {
  const setScopes = useAuthStore((s) => s.setScopes);

  useEffect(() => {
    const tick = async () => {
      try {
        const scopes = await bridge.getScopeStatus();
        setScopes(scopes);
      } catch {
        /* signed out or invoke error */
      }
    };
    tick();
    const id = window.setInterval(tick, 30_000);
    return () => window.clearInterval(id);
  }, [setScopes]);

  return null;
}

export function AppShell() {
  const scopes = useAuthStore((s) => s.scopes);
  const appLocked = scopes !== null && !scopes.app;

  return (
    <div className="flex h-dvh overflow-hidden bg-bg">
      <SessionWatchdog />
      {!appLocked && <Sidebar />}
      <main className="min-h-0 min-w-0 flex-1 overflow-y-auto p-6 pb-20">
        <Outlet />
      </main>
      <AppLockModal open={appLocked} />
    </div>
  );
}
