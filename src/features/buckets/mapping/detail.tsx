import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Eye, EyeOff, Trash2 } from "lucide-react";

import { bridge } from "@/core/bridge";
import { toast } from "@/core/toast";
import type { BucketMapping } from "@/shared/types/bucket";
import type { SecretMeta } from "@/shared/types/secret";
import { ArgusInput } from "@/shared/ui/argus-input";
import { Button } from "@/shared/ui/button";
import { Switch } from "@/shared/ui/switch";
import { MappingAllowedHosts } from "@/features/buckets/mapping/hosts";
import { SecretPicker } from "@/features/buckets/picker";

type MappingType = "secret" | "text";

type FormFields = {
  envLabel: string;
  mappingType: MappingType;
  secretId: string;
  textValue: string;
  proxyEnabled: boolean;
  allowedHosts: string[];
};

interface BucketMappingDetailPanelProps {
  bucketId: string;
  mapping: BucketMapping | null;
  isDraft: boolean;
  secrets: SecretMeta[];
  proxyBucketEnabled: boolean;
  onDelete?: () => void;
  onSaved: (saved: BucketMapping) => void;
  onCancelDraft?: () => void;
}

function canPersist(fields: FormFields): boolean {
  if (!fields.envLabel.trim()) return false;
  if (fields.mappingType === "secret" && !fields.secretId) return false;
  if (fields.mappingType === "text" && !fields.textValue.trim()) return false;
  return true;
}

function formSnapshot(fields: FormFields, proxyBucketEnabled: boolean): string {
  return JSON.stringify({
    envLabel: fields.envLabel.trim(),
    mappingType: fields.mappingType,
    secretId: fields.secretId,
    textValue: fields.textValue.trim(),
    proxyEnabled: proxyBucketEnabled && fields.proxyEnabled,
    allowedHosts: fields.allowedHosts,
  });
}

function fieldsFromMapping(
  mapping: BucketMapping,
  patch?: Partial<FormFields>,
): FormFields {
  return {
    envLabel: mapping.envLabel,
    mappingType: mapping.mappingType,
    secretId: mapping.secretId ?? "",
    textValue: mapping.textValue ?? "",
    proxyEnabled: mapping.proxyEnabled,
    allowedHosts: mapping.allowedHosts,
    ...patch,
  };
}

function emptyFields(): FormFields {
  return {
    envLabel: "",
    mappingType: "secret",
    secretId: "",
    textValue: "",
    proxyEnabled: false,
    allowedHosts: [],
  };
}

export function BucketMappingDetailPanel({
  bucketId,
  mapping,
  isDraft,
  secrets,
  proxyBucketEnabled,
  onDelete,
  onSaved,
  onCancelDraft,
}: BucketMappingDetailPanelProps) {
  const [envLabel, setEnvLabel] = useState("");
  const [mappingType, setMappingType] = useState<MappingType>("secret");
  const [secretId, setSecretId] = useState("");
  const [textValue, setTextValue] = useState("");
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [allowedHosts, setAllowedHosts] = useState<string[]>([]);
  const [savedProxyToken, setSavedProxyToken] = useState<string | null>(null);
  const [tokenRevealed, setTokenRevealed] = useState(false);
  const [saving, setSaving] = useState(false);

  const lastPersisted = useRef("");
  const onSavedRef = useRef(onSaved);
  onSavedRef.current = onSaved;

  const currentFields: FormFields = useMemo(
    () => ({
      envLabel,
      mappingType,
      secretId,
      textValue,
      proxyEnabled,
      allowedHosts,
    }),
    [envLabel, mappingType, secretId, textValue, proxyEnabled, allowedHosts],
  );

  const mappingId = mapping?.id ?? null;
  const hydrateKey = isDraft ? "draft" : mappingId;
  const envNameLocked = !isDraft && !!mapping;

  const dirty = formSnapshot(currentFields, proxyBucketEnabled) !== lastPersisted.current;

  const applyFields = useCallback((fields: FormFields, proxyToken: string | null) => {
    setEnvLabel(fields.envLabel);
    setMappingType(fields.mappingType);
    setSecretId(fields.secretId);
    setTextValue(fields.textValue);
    setProxyEnabled(fields.proxyEnabled);
    setAllowedHosts(fields.allowedHosts);
    setSavedProxyToken(proxyToken);
    setTokenRevealed(false);
  }, []);

  useEffect(() => {
    if (mapping) {
      const fields = fieldsFromMapping(mapping);
      applyFields(fields, mapping.proxyPlaceholder);
      lastPersisted.current = formSnapshot(fields, proxyBucketEnabled);
    } else if (isDraft) {
      applyFields(emptyFields(), null);
      lastPersisted.current = formSnapshot(emptyFields(), proxyBucketEnabled);
    }
  }, [hydrateKey, isDraft, proxyBucketEnabled, applyFields]);

  const persist = useCallback(async () => {
    const fields = currentFields;
    if (!canPersist(fields)) return;

    setSaving(true);
    try {
      const saved = await bridge.upsertBucketMapping({
        bucketId,
        envLabel: fields.envLabel.trim(),
        mappingType: fields.mappingType,
        secretId: fields.mappingType === "secret" ? fields.secretId : undefined,
        textValue: fields.mappingType === "text" ? fields.textValue.trim() : undefined,
        proxyEnabled: proxyBucketEnabled && fields.proxyEnabled,
        allowedHosts: fields.allowedHosts,
      });
      lastPersisted.current = formSnapshot(fields, proxyBucketEnabled);
      setSavedProxyToken(saved.proxyPlaceholder);
      onSavedRef.current(saved);
      toast.success(isDraft ? `${saved.envLabel} created` : `${saved.envLabel} updated`);
    } catch (e) {
      toast.fromError(e, "Failed to save mapping");
    } finally {
      setSaving(false);
    }
  }, [bucketId, proxyBucketEnabled, currentFields, isDraft]);

  function handleCancel() {
    if (isDraft) {
      onCancelDraft?.();
      return;
    }
    if (mapping) {
      const fields = fieldsFromMapping(mapping);
      applyFields(fields, mapping.proxyPlaceholder);
      lastPersisted.current = formSnapshot(fields, proxyBucketEnabled);
    }
  }

  function handleProxyToggle(enabled: boolean) {
    setProxyEnabled(enabled);
    setTokenRevealed(false);
    if (!enabled) {
      setSavedProxyToken(null);
    }
  }

  if (!mapping && !isDraft) {
    return (
      <div className="rounded-xl border border-border bg-surface p-6">
        <p className="text-sm text-text-muted">Select an env key to view or edit.</p>
      </div>
    );
  }

  const showProxyToken = proxyBucketEnabled && proxyEnabled;
  const maskedToken =
    savedProxyToken != null
      ? "•".repeat(Math.min(Math.max(savedProxyToken.length, 16), 40))
      : "";

  return (
    <div className="rounded-xl border border-border bg-surface p-6">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 className="text-sm font-semibold">{isDraft ? "New mapping" : "Mapping details"}</h2>
          {saving && <p className="mt-0.5 text-xs text-text-muted">Saving…</p>}
        </div>
        {!isDraft && mapping && onDelete && (
          <Button
            type="button"
            variant="danger"
            className="h-8 shrink-0 px-3 text-xs"
            onClick={onDelete}
            aria-label="Delete mapping"
          >
            <Trash2 className="size-3.5" aria-hidden />
          </Button>
        )}
      </div>

      <div className="mt-4 space-y-4">
        <div>
          <label className="text-xs font-medium text-text-muted">Env name</label>
          <ArgusInput
            className="mt-1 font-mono"
            value={envLabel}
            onChange={(e) => setEnvLabel(e.target.value)}
            placeholder="OPENAI_API_KEY"
            disabled={envNameLocked}
            readOnly={envNameLocked}
          />
          {envNameLocked && (
            <p className="mt-1 text-xs text-text-muted">Env name cannot be changed after creation.</p>
          )}
        </div>

        <div>
          <label className="text-xs font-medium text-text-muted">Type</label>
          <select
            className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
            value={mappingType}
            onChange={(e) => setMappingType(e.target.value as MappingType)}
            aria-label="Mapping type"
          >
            <option value="secret">Vault secret</option>
            <option value="text">Text value</option>
          </select>
        </div>

        <div>
          <label className="text-xs font-medium text-text-muted">Value</label>
          {mappingType === "secret" ? (
            <div className="mt-1">
              <SecretPicker
                secrets={secrets}
                value={secretId}
                onChange={setSecretId}
              />
            </div>
          ) : (
            <ArgusInput
              className="mt-1"
              value={textValue}
              onChange={(e) => setTextValue(e.target.value)}
              placeholder="Plain text value"
            />
          )}
        </div>

        {proxyBucketEnabled && (
          <>
            <div className="flex items-center justify-between rounded-lg border border-border px-3 py-2">
              <p className="text-sm font-medium">Inject proxy token</p>
              <Switch
                checked={proxyEnabled}
                onChange={handleProxyToggle}
                aria-label="Inject proxy token"
              />
            </div>

            {showProxyToken && (
              <div className="space-y-3">
                {savedProxyToken ? (
                  <div className="flex items-center gap-2 rounded-md border border-border bg-surface-raised/60 px-3 py-2">
                    <code className="block min-w-0 flex-1 break-all py-0.5 font-mono text-xs leading-snug text-text">
                      {tokenRevealed ? savedProxyToken : maskedToken}
                    </code>
                    <button
                      type="button"
                      aria-label={tokenRevealed ? "Hide proxy token" : "Show proxy token"}
                      title={tokenRevealed ? "Hide" : "Show"}
                      onClick={() => setTokenRevealed((v) => !v)}
                      className="grid size-7 shrink-0 place-items-center rounded text-text-muted transition-colors hover:bg-surface-raised hover:text-text"
                    >
                      {tokenRevealed ? (
                        <EyeOff className="size-3.5" aria-hidden />
                      ) : (
                        <Eye className="size-3.5" aria-hidden />
                      )}
                    </button>
                  </div>
                ) : (
                  <p className="text-xs text-text-muted">
                    Save this mapping to generate a proxy token.
                  </p>
                )}

                <MappingAllowedHosts
                  hosts={allowedHosts}
                  onChange={setAllowedHosts}
                  disabled={saving}
                />
              </div>
            )}
          </>
        )}

        {dirty && (
          <div className="flex gap-2 border-t border-border pt-4">
            <Button
              type="button"
              variant="secondary"
              className="flex-1"
              onClick={handleCancel}
              disabled={saving}
            >
              Cancel
            </Button>
            <Button
              type="button"
              variant="primary"
              className="flex-1"
              onClick={() => void persist()}
              disabled={saving || !canPersist(currentFields)}
            >
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
