import { LayoutDashboard } from "lucide-react";

import { Card } from "../ui/card";
import { Stack } from "../ui/stack";
import { Text } from "../ui/text";

export function DashboardWelcome() {
  return (
    <Stack>
      <div className="flex items-center gap-3">
        <LayoutDashboard className="text-accent" size={28} />
        <h1 className="text-2xl font-semibold text-text">Dashboard</h1>
      </div>
      <Text tone="muted" className="max-w-lg">
        Welcome to Argus. Your local account is secured with encryption and
        two-factor authentication. Vault and buckets will appear here in a future
        update.
      </Text>
      <Card>
        <h2 className="text-lg font-medium text-text">Getting started</h2>
        <ul className="mt-3 list-inside list-disc space-y-1 text-sm text-text-muted">
          <li>Session active with APP scope</li>
          <li>Sign out from Settings when you are done</li>
        </ul>
      </Card>
    </Stack>
  );
}
