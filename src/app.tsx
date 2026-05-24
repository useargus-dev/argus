import { useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppShell } from "./components/layout/app-shell";
import { ArgusToaster } from "./components/ui/argus-toaster";
import { ThemeToggle } from "./components/layout/theme-toggle";
import { useTauriEvent } from "./hooks/use-tauri-event";
import { bridge } from "./lib/tauri-bridge";
import { DashboardPage } from "./pages/dashboard";
import { LoginPage } from "./pages/login";
import { RegisterPage } from "./pages/register";
import { RegisterProvisioningPage } from "./pages/register-provisioning";
import { SettingsPage } from "./pages/settings";
import { VaultPage } from "./pages/vault";
import { useAuthStore } from "./state/auth-store";
import { getStoredTheme } from "./lib/theme";
import { useThemeStore } from "./state/theme-store";
import type { ScopeStatus, UserProfile } from "./types/auth";

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

  return (
    <BrowserRouter>
      <ArgusToaster />
      <ThemeToggle />
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
        <Route
          element={
            <RequireAuth>
              <AppShell />
            </RequireAuth>
          }
        >
          <Route path="/dashboard" element={<DashboardPage />} />
          <Route path="/vault" element={<VaultPage />} />
          <Route path="/settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
