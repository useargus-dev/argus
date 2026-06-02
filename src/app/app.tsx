import { useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "@/shared/layout/app-shell";
import { ArgusToaster } from "@/shared/ui/argus-toaster";
import { ThemeToggle } from "@/shared/layout/theme-toggle";
import { useTauriEvent } from "@/shared/hooks/event";
import { bridge } from "@/core/bridge";
import { ApprovalsPage } from "@/features/approvals/page";
import { BucketDetailPage } from "@/features/buckets/detail-page";
import { BucketsPage } from "@/features/buckets/page";
import { RequestsPage } from "@/features/clients/requests";
import { DashboardPage } from "@/features/dashboard/page";
import { LoginPage } from "@/features/login/page";
import { RegisterPage } from "@/features/register/page";
import { RegisterProvisioningPage } from "@/features/register/provision";
import { SettingsPage } from "@/features/settings/page";
import { VaultPage } from "@/features/secrets/page";
import { useAuthStore } from "@/state/auth-store";
import { getStoredTheme } from "@/core/theme";
import { useThemeStore } from "@/state/theme-store";
import type { ScopeStatus, UserProfile } from "@/shared/types/auth";

function RequireAuth({ children }: { children: React.ReactNode }) {
  const isSignedIn = useAuthStore((s) => s.isSignedIn);
  if (!isSignedIn) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function RequireNoAccount({ children }: { children: React.ReactNode }) {
  const [hasAccount, setHasAccount] = useState<boolean | null>(null);

  useEffect(() => {
    bridge
      .hasAccount()
      .then(setHasAccount)
      .catch(() => setHasAccount(false));
  }, []);

  if (hasAccount === null) return null;
  if (hasAccount) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function RootRedirect() {
  const [hasAccount, setHasAccount] = useState<boolean | null>(null);
  const isSignedIn = useAuthStore((s) => s.isSignedIn);

  useEffect(() => {
    bridge
      .hasAccount()
      .then(setHasAccount)
      .catch(() => setHasAccount(false));
  }, []);

  if (hasAccount === null) return null;
  if (isSignedIn) return <Navigate to="/dashboard" replace />;
  if (!hasAccount) return <Navigate to="/register" replace />;
  return <Navigate to="/login" replace />;
}

export default function App() {
  const setSignedIn = useAuthStore((s) => s.setSignedIn);
  const clear = useAuthStore((s) => s.clear);
  const setScopes = useAuthStore((s) => s.setScopes);
  useEffect(() => {
    useThemeStore.setState({ theme: getStoredTheme() });
  }, []);

  useTauriEvent<UserProfile>("signed-in", (p) => {
    bridge.getScopeStatus().then((scopes) => setSignedIn(p, scopes));
  });

  useTauriEvent("signed-out", () => {
    clear();
  });

  useTauriEvent<ScopeStatus>("scope-changed", setScopes);

  useTauriEvent("app-locked", () => {
    bridge.getScopeStatus().then(setScopes).catch(() => {});
  });

  const isRequestsWindow = window.location.pathname === "/requests";

  return (
    <BrowserRouter>
      <ArgusToaster />
      {!isRequestsWindow && <ThemeToggle />}
      <Routes>
        <Route path="/" element={<RootRedirect />} />
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/register"
          element={
            <RequireNoAccount>
              <RegisterPage />
            </RequireNoAccount>
          }
        />
        <Route
          path="/register/provisioning"
          element={
            <RequireNoAccount>
              <RegisterProvisioningPage />
            </RequireNoAccount>
          }
        />
        <Route path="/requests" element={<RequestsPage />} />
        <Route
          element={
            <RequireAuth>
              <AppShell />
            </RequireAuth>
          }
        >
          <Route path="/dashboard" element={<DashboardPage />} />
          <Route path="/vault" element={<VaultPage />} />
          <Route path="/buckets" element={<BucketsPage />} />
          <Route path="/approvals" element={<ApprovalsPage />} />
          <Route path="/buckets/:id" element={<BucketDetailPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
