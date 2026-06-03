import { useEffect, useState } from "react";
import { User } from "lucide-react";
import type { UserProfile } from "@/shared/types/auth";
import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import { useAuthStore } from "@/state/auth-store";
import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { SettingsRow } from "@/features/settings/row";
import { SettingsSection } from "@/features/settings/section";

interface ProfileSectionProps {
  profile: UserProfile | null;
}

export function ProfileSection({ profile }: ProfileSectionProps) {
  const setSignedIn = useAuthStore((s) => s.setSignedIn);
  const scopes = useAuthStore((s) => s.scopes);
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setFirstName(profile?.firstName ?? "");
    setLastName(profile?.lastName ?? "");
    setUsername(profile?.username ?? "");
    setEmail(
      profile?.email && profile.email !== "unset@local.argus" ? profile.email : "",
    );
  }, [profile]);

  async function handleSave() {
    if (!profile || !scopes) return;
    setSaving(true);
    try {
      const updated = await bridge.updateProfile({
        firstName: firstName.trim(),
        lastName: lastName.trim(),
        ...(username.trim() ? { username: username.trim() } : {}),
        ...(email.trim() ? { email: email.trim() } : {}),
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
    firstName !== (profile?.firstName ?? "") ||
    lastName !== (profile?.lastName ?? "") ||
    username !== (profile?.username ?? "") ||
    email !==
      (profile?.email && profile.email !== "unset@local.argus"
        ? profile.email
        : "");

  return (
    <SettingsSection title="Profile" icon={User}>
      <SettingsRow label="First name">
        <ArgusInput
          className="min-w-[12rem] flex-1"
          value={firstName}
          onChange={(e) => setFirstName(e.target.value)}
        />
      </SettingsRow>
      <SettingsRow label="Last name">
        <ArgusInput
          className="min-w-[12rem] flex-1"
          value={lastName}
          onChange={(e) => setLastName(e.target.value)}
        />
      </SettingsRow>
      <SettingsRow label="Username">
        <ArgusInput
          className="min-w-[12rem] flex-1"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          placeholder="Optional"
        />
      </SettingsRow>
      <SettingsRow label="Email">
        <ArgusInput
          className="min-w-[12rem] flex-1"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="Optional"
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
