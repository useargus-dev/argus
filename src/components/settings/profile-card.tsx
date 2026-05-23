import { Card } from "../ui/card";
import type { UserProfile } from "../../types/auth";

export function SettingsProfileCard({ profile }: { profile: UserProfile | null }) {
  return (
    <Card>
      <h2 className="text-lg font-medium text-text">Profile</h2>
      <dl className="mt-4 space-y-2 text-sm">
        <div className="flex justify-between gap-4">
          <dt className="text-text-muted">Username</dt>
          <dd className="text-text">{profile?.username ?? "—"}</dd>
        </div>
        <div className="flex justify-between gap-4">
          <dt className="text-text-muted">Email</dt>
          <dd className="text-text">{profile?.email ?? "—"}</dd>
        </div>
      </dl>
    </Card>
  );
}
