import type { LucideIcon } from "lucide-react";
import {
  Database,
  FileBadge,
  Key,
  Lock,
  Shield,
  StickyNote,
  Terminal,
  Ticket,
} from "lucide-react";

import type { SecretMeta, SecretType } from "../types/secret";

export const SECRET_TYPE_OPTIONS: { value: SecretType; label: string }[] = [
  { value: "api_key", label: "API key" },
  { value: "access_token", label: "Access token" },
  { value: "credential", label: "Credential" },
  { value: "recovery_codes", label: "Recovery codes" },
  { value: "ssh_key", label: "SSH key" },
  { value: "certificate", label: "Certificate" },
  { value: "connection_string", label: "Connection string" },
  { value: "note", label: "Note" },
];

const TYPE_ICONS: Record<string, LucideIcon> = {
  api_key: Key,
  access_token: Ticket,
  credential: Lock,
  recovery_codes: Shield,
  ssh_key: Terminal,
  certificate: FileBadge,
  connection_string: Database,
  note: StickyNote,
  password: Lock,
};

const TYPE_LABELS: Record<string, string> = Object.fromEntries(
  SECRET_TYPE_OPTIONS.map(({ value, label }) => [value, label]),
);
TYPE_LABELS.password = "Credential";

export function secretTypeIcon(type: SecretType): LucideIcon {
  return TYPE_ICONS[type] ?? Key;
}

export function secretTypeLabel(type: SecretType): string {
  return TYPE_LABELS[type] ?? type.replace(/_/g, " ");
}

export function readSecretValue(value: Record<string, string>): string {
  if (value.value) return value.value;
  if (value.password) return value.password;
  if (value.apiKey) return value.apiKey;
  if (value.note) return value.note;
  const first = Object.values(value)[0];
  return first ?? "";
}

export function writeSecretValue(plain: string): Record<string, string> {
  return { value: plain };
}

export function secretSubtitle(meta: SecretMeta): string {
  const parts = [meta.organization, meta.environment].filter(Boolean);
  return parts.length > 0 ? parts.join(" · ") : secretTypeLabel(meta.secretType);
}

export interface VaultFilters {
  query: string;
  types: SecretType[];
  environment: string;
  tags: string[];
}

export function collectFilterOptions(secrets: SecretMeta[]) {
  const environments = new Set<string>();
  const tags = new Set<string>();
  for (const s of secrets) {
    if (s.environment?.trim()) environments.add(s.environment.trim());
    for (const t of s.tags) {
      if (t.trim()) tags.add(t.trim());
    }
  }
  return {
    environments: [...environments].sort((a, b) => a.localeCompare(b)),
    tags: [...tags].sort((a, b) => a.localeCompare(b)),
  };
}

export function filterSecrets(
  secrets: SecretMeta[],
  filters: VaultFilters,
): SecretMeta[] {
  const q = filters.query.trim().toLowerCase();

  return secrets.filter((s) => {
    if (filters.types.length > 0 && !filters.types.includes(s.secretType)) {
      return false;
    }
    if (
      filters.environment !== "all" &&
      (s.environment ?? "").trim() !== filters.environment
    ) {
      return false;
    }
    if (filters.tags.length > 0) {
      const secretTags = new Set(s.tags.map((t) => t.toLowerCase()));
      const matchesTag = filters.tags.some((t) =>
        secretTags.has(t.toLowerCase()),
      );
      if (!matchesTag) return false;
    }
    if (!q) return true;

    const haystack = [
      s.name,
      s.description ?? "",
      s.organization ?? "",
      s.environment ?? "",
      ...s.tags,
    ]
      .join(" ")
      .toLowerCase();

    return haystack.includes(q);
  });
}

export function daysUntilExpiry(expiresAt: string | null): number | null {
  if (!expiresAt) return null;
  const end = new Date(expiresAt);
  if (Number.isNaN(end.getTime())) return null;
  const diff = end.getTime() - Date.now();
  return Math.ceil(diff / (1000 * 60 * 60 * 24));
}

export function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString();
}

export function expiryBadgeTone(days: number): "danger" | "warning" | null {
  if (days < 0) return "danger";
  if (days <= 7) return "danger";
  if (days <= 30) return "warning";
  return null;
}
