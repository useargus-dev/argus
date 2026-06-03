import { LogOut } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { SettingsSection } from "@/features/settings/section";

interface SignOutSectionProps {
  signingOut: boolean;
  onSignOut: () => void;
}

export function SignOutSection({ signingOut, onSignOut }: SignOutSectionProps) {
  return (
    <SettingsSection title="Session" icon={LogOut} variant="danger">
      <p className="-mt-2 mb-2 text-xs text-text-muted">
        Clears encryption keys from memory. You will need your password and second
        factor the next time you open Argus.
      </p>
      <Button
        type="button"
        variant="danger"
        className="h-10 w-full gap-2 border border-danger/30"
        onClick={onSignOut}
        disabled={signingOut}
      >
        <LogOut className="size-4" aria-hidden />
        {signingOut ? "Signing out…" : "Sign out"}
      </Button>
    </SettingsSection>
  );
}
