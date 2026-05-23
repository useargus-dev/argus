import { Button } from "../ui/button";
import { Card } from "../ui/card";
import { Text } from "../ui/text";

interface SessionCardProps {
  signingOut: boolean;
  onSignOut: () => void;
}

export function SettingsSessionCard({ signingOut, onSignOut }: SessionCardProps) {
  return (
    <Card>
      <h2 className="text-lg font-medium text-text">Session</h2>
      <Text tone="muted" className="mt-2">
        Sign out clears encryption keys from memory and locks the vault.
      </Text>
      <Button
        type="button"
        variant="danger"
        className="mt-4 w-full"
        onClick={onSignOut}
        disabled={signingOut}
      >
        {signingOut ? "Signing out…" : "Sign out"}
      </Button>
    </Card>
  );
}
