import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "@/core/toast";

import { AboutSection } from "@/features/settings/about";
import { AuthenticationSection } from "@/features/settings/auth";
import { BackgroundSection } from "@/features/settings/bg";
import { NotificationsSection } from "@/features/settings/notify";
import { ProfileSection } from "@/features/settings/profile";
import { SecuritySection } from "@/features/settings/security";
import { SignOutSection } from "@/features/settings/signout";
import { bridge } from "@/core/bridge";
import { useAuthStore } from "@/state/auth-store";
import type { SecondFactorStatus } from "@/shared/types/settings";

export function SettingsPage() {
  const navigate = useNavigate();
  const profile = useAuthStore((s) => s.profile);
  const clear = useAuthStore((s) => s.clear);
  const [signingOut, setSigningOut] = useState(false);
  const [factorStatus, setFactorStatus] = useState<SecondFactorStatus | null>(
    null,
  );

  useEffect(() => {
    bridge
      .getSecondFactorStatus()
      .then(setFactorStatus)
      .catch(() => toast.error("Could not load authentication methods"));
  }, []);

  async function handleSignOut() {
    if (!confirm("Sign out and return to the sign-in screen?")) return;
    setSigningOut(true);
    try {
      await bridge.signOut();
      clear();
      navigate("/login", { replace: true });
    } catch (e) {
      toast.fromError(e, "Sign out failed");
    } finally {
      setSigningOut(false);
    }
  }

  return (
    <div className="mx-auto max-w-3xl px-2 py-2">
      <header className="mb-6 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight text-text">
            Settings
          </h1>
          <p className="mt-1 text-sm text-text-muted">Local app preferences.</p>
        </div>
      </header>

      <div className="space-y-4">
        <ProfileSection profile={profile} />
        <SecuritySection
          factorStatus={factorStatus}
          onFactorStatusChange={setFactorStatus}
        />
        <AuthenticationSection
          factorStatus={factorStatus}
          onFactorStatusChange={setFactorStatus}
        />
        <BackgroundSection />
        <NotificationsSection />
        <AboutSection />
        <SignOutSection signingOut={signingOut} onSignOut={handleSignOut} />
      </div>
    </div>
  );
}
