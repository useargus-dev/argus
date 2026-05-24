import { useEffect, useState } from "react";
import { X } from "lucide-react";

import {
  readSecretValue,
  SECRET_TYPE_OPTIONS,
  writeSecretValue,
} from "../../lib/secret-utils";
import type { SecretDetail, SecretType, SecretWriteInput } from "../../types/secret";
import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { Field } from "../ui/field";
import { PasswordInput } from "../ui/password-input";
import { Textarea } from "../ui/textarea";
import { SecretBadge } from "./secret-badge";

interface SecretFormDialogProps {
  open: boolean;
  initial?: SecretDetail | null;
  saving: boolean;
  onClose: () => void;
  onSave: (input: SecretWriteInput) => void;
}

function parseTags(raw: string): string[] {
  return raw
    .split(/[,\s]+/)
    .map((t) => t.replace(/^#/, "").trim())
    .filter(Boolean);
}

function SecretValueField({
  secretType,
  value,
  onChange,
}: {
  secretType: SecretType | "";
  value: string;
  onChange: (v: string) => void;
}) {
  const isCredential = secretType === "credential" || secretType === "password";
  const isLongText =
    secretType === "note" || secretType === "recovery_codes";

  if (isCredential) {
    return (
      <PasswordInput
        label="Secret value"
        value={value}
        onChange={onChange}
        placeholder="Required"
      />
    );
  }

  return (
    <Field label="Secret value">
      <Textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        rows={isLongText ? 5 : 4}
        required
        className={isLongText ? undefined : "font-mono text-xs"}
        placeholder={
          isLongText
            ? "Required — paste note or recovery codes"
            : "Required — paste key, token, connection string…"
        }
      />
    </Field>
  );
}

export function SecretFormDialog({
  open,
  initial,
  saving,
  onClose,
  onSave,
}: SecretFormDialogProps) {
  const [name, setName] = useState("");
  const [secretType, setSecretType] = useState<SecretType | "">("");
  const [organization, setOrganization] = useState("");
  const [environment, setEnvironment] = useState("");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState("");
  const [expiresAt, setExpiresAt] = useState("");
  const [value, setValue] = useState("");

  useEffect(() => {
    if (!open) return;
    if (initial) {
      setName(initial.name);
      setSecretType(initial.secretType);
      setOrganization(initial.organization ?? "");
      setEnvironment(initial.environment ?? "");
      setDescription(initial.description ?? "");
      setTags(initial.tags);
      setTagInput("");
      setExpiresAt(
        initial.expiresAt ? initial.expiresAt.slice(0, 10) : "",
      );
      setValue(readSecretValue(initial.value));
    } else {
      setName("");
      setSecretType("");
      setOrganization("");
      setEnvironment("");
      setDescription("");
      setTags([]);
      setTagInput("");
      setExpiresAt("");
      setValue("");
    }
  }, [open, initial]);

  if (!open) return null;

  const canSubmit =
    name.trim().length > 0 &&
    value.trim().length > 0 &&
    secretType.trim().length > 0;

  function addTag(raw: string) {
    const next = parseTags(raw);
    if (next.length === 0) return;
    setTags((prev) => {
      const set = new Set(prev.map((t) => t.toLowerCase()));
      const merged = [...prev];
      for (const t of next) {
        if (!set.has(t.toLowerCase())) {
          merged.push(t);
          set.add(t.toLowerCase());
        }
      }
      return merged;
    });
    setTagInput("");
  }

  function removeTag(tag: string) {
    setTags((prev) => prev.filter((t) => t !== tag));
  }

  function handleTagKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      addTag(tagInput);
    } else if (e.key === "Backspace" && !tagInput && tags.length > 0) {
      setTags((prev) => prev.slice(0, -1));
    }
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSubmit) return;

    const payload: SecretWriteInput = {
      name: name.trim(),
      secretType: secretType as SecretType,
      organization: organization.trim() || undefined,
      environment: environment.trim() || undefined,
      description: description.trim() || undefined,
      tags: tags.length > 0 ? tags : undefined,
      expiresAt: expiresAt ? new Date(expiresAt).toISOString() : undefined,
      value: writeSecretValue(value.trim()),
    };
    onSave(payload);
  }

  const selectClassName =
    "h-9 w-full rounded-md border border-border bg-surface px-3 text-sm focus:border-accent focus:outline-none";

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-bg/80 p-4 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="secret-form-title"
    >
      <div className="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-xl border border-border bg-surface p-6 shadow-lg">
        <div className="flex items-start justify-between gap-4">
          <h2 id="secret-form-title" className="text-lg font-semibold text-text">
            {initial ? "Edit secret" : "Add secret"}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="rounded-md p-1 text-text-muted hover:bg-surface-raised hover:text-text"
            aria-label="Close"
          >
            <X className="size-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="mt-4 space-y-4">
          <Field label="Name">
            <ArgusInput
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              autoFocus
              placeholder="Required"
            />
          </Field>

          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Type">
              <select
                className={selectClassName}
                value={secretType}
                onChange={(e) => setSecretType(e.target.value as SecretType | "")}
                required
              >
                <option value="" disabled>
                  Select type…
                </option>
                {SECRET_TYPE_OPTIONS.map(({ value: v, label }) => (
                  <option key={v} value={v}>
                    {label}
                  </option>
                ))}
                {initial?.secretType === "password" &&
                  !SECRET_TYPE_OPTIONS.some((o) => o.value === "password") && (
                    <option value="password">Credential (legacy)</option>
                  )}
              </select>
            </Field>
            <Field label="Expires (optional)">
              <ArgusInput
                type="date"
                value={expiresAt}
                onChange={(e) => setExpiresAt(e.target.value)}
              />
            </Field>
          </div>

          <SecretValueField
            secretType={secretType}
            value={value}
            onChange={setValue}
          />

          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Organization (optional)">
              <ArgusInput
                value={organization}
                onChange={(e) => setOrganization(e.target.value)}
                placeholder="Acme"
              />
            </Field>
            <Field label="Environment (optional)">
              <ArgusInput
                value={environment}
                onChange={(e) => setEnvironment(e.target.value)}
                placeholder="prod, dev…"
              />
            </Field>
          </div>

          <Field label="Tags (optional)">
            <div className="rounded-md border border-border bg-surface-raised px-2 py-1.5">
              <div className="flex flex-wrap gap-1.5">
                {tags.map((tag) => (
                  <button
                    key={tag}
                    type="button"
                    onClick={() => removeTag(tag)}
                    aria-label={`Remove tag ${tag}`}
                  >
                    <SecretBadge prefix="#">{tag}</SecretBadge>
                  </button>
                ))}
                <input
                  value={tagInput}
                  onChange={(e) => setTagInput(e.target.value)}
                  onKeyDown={handleTagKeyDown}
                  onBlur={() => tagInput.trim() && addTag(tagInput)}
                  placeholder={tags.length === 0 ? "Add tags (Enter)" : ""}
                  className="min-w-[8rem] flex-1 bg-transparent py-0.5 text-sm outline-none placeholder:text-text-muted"
                />
              </div>
            </div>
          </Field>

          <Field label="Description (optional)">
            <Textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
              placeholder="Notes about this secret"
            />
          </Field>

          <div className="flex gap-2 pt-2">
            <Button
              type="button"
              variant="ghost"
              className="flex-1"
              onClick={onClose}
              disabled={saving}
            >
              Cancel
            </Button>
            <Button
              type="submit"
              variant="primary"
              className="flex-1"
              disabled={saving || !canSubmit}
            >
              {saving ? "Saving…" : initial ? "Save changes" : "Add secret"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
