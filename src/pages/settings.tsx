import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { SettingsProfileCard } from "../components/settings/profile-card";
import { SettingsSessionCard } from "../components/settings/session-card";
import { Stack } from "../components/ui/stack";
import { bridge } from "../lib/tauri-bridge";
import { useAuthStore } from "../state/auth-store";

export function SettingsPage() {
  const navigate = useNavigate();
  const profile = useAuthStore((s) => s.profile);
  const clear = useAuthStore((s) => s.clear);
  const [signingOut, setSigningOut] = useState(false);

  async function handleSignOut() {
    if (!confirm("Lock vault and return to sign in?")) return;
    setSigningOut(true);
    try {
      await bridge.signOut();
      clear();
      navigate("/login", { replace: true });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : "Sign out failed");
    } finally {
      setSigningOut(false);
    }
  }

  return (
    <Stack className="mx-auto max-w-xl">
      <h1 className="text-2xl font-semibold text-text">Settings</h1>
      <SettingsProfileCard profile={profile} />
      <SettingsSessionCard signingOut={signingOut} onSignOut={handleSignOut} />
    </Stack>
  );
}
