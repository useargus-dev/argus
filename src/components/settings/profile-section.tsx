import { useEffect, useState } from "react";
import { User } from "lucide-react";
import type { UserProfile } from "../../types/auth";
import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import { useAuthStore } from "../../state/auth-store";
import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { SettingsRow } from "./settings-row";
import { SettingsSection } from "./settings-section";

interface ProfileSectionProps {
  profile: UserProfile | null;
}

export function ProfileSection({ profile }: ProfileSectionProps) {
  const setSignedIn = useAuthStore((s) => s.setSignedIn);
  const scopes = useAuthStore((s) => s.scopes);
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setUsername(profile?.username ?? "");
    setEmail(profile?.email ?? "");
  }, [profile]);

  async function handleSave() {
    if (!profile || !scopes) return;
    setSaving(true);
    try {
      const updated = await bridge.updateProfile({
        username: username.trim(),
        email: email.trim(),
      });
      setSignedIn(updated, scopes);
      toast.success("Profile updated");
    } catch (e) {
      toast.fromError(e, "Failed to save profile");
    } finally {
      setSaving(false);
    }
  }

  const dirty =
    username !== (profile?.username ?? "") || email !== (profile?.email ?? "");

  return (
    <SettingsSection title="Profile" icon={User}>
      <SettingsRow label="Username">
        <ArgusInput
          className="min-w-[12rem] flex-1"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
        />
      </SettingsRow>
      <SettingsRow label="Email">
        <ArgusInput
          className="min-w-[12rem] flex-1"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
        />
      </SettingsRow>
      <Button
        type="button"
        variant="secondary"
        className="mt-2 w-full sm:w-auto"
        disabled={!dirty || saving}
        onClick={handleSave}
      >
        {saving ? "Saving…" : "Save profile"}
      </Button>
    </SettingsSection>
  );
}
