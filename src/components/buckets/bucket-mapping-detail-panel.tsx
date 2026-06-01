import { useCallback, useEffect, useRef, useState } from "react";
import { Eye, EyeOff, Trash2 } from "lucide-react";

import { bridge } from "../../lib/tauri-bridge";
import { generateProxyToken } from "../../lib/proxy-token";
import { toast } from "../../lib/toast";
import type { BucketMapping } from "../../types/bucket";
import type { SecretMeta } from "../../types/secret";
import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { Switch } from "../settings/switch";
import { MappingAllowedHosts } from "./mapping-allowed-hosts";
import { SecretPicker } from "./secret-picker";

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
  /** Plaintext proxy tokens already used by other mappings in this bucket. */
  bucketProxyTokens: ReadonlySet<string>;
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

function notifyMappingUpdated(saved: BucketMapping) {
  toast.success(`${saved.envLabel} Updated`);
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

export function BucketMappingDetailPanel({
  bucketId,
  mapping,
  isDraft,
  secrets,
  proxyBucketEnabled,
  bucketProxyTokens,
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
  const [proxyToken, setProxyToken] = useState<string | null>(null);
  const [tokenRevealed, setTokenRevealed] = useState(false);
  const [saving, setSaving] = useState(false);

  const lastPersisted = useRef("");
  const onSavedRef = useRef(onSaved);
  onSavedRef.current = onSaved;

  const formRef = useRef<FormFields>({
    envLabel: "",
    mappingType: "secret",
    secretId: "",
    textValue: "",
    proxyEnabled: false,
    allowedHosts: [],
  });

  formRef.current = {
    envLabel,
    mappingType,
    secretId,
    textValue,
    proxyEnabled,
    allowedHosts,
  };

  const mappingId = mapping?.id ?? null;
  const hydrateKey = isDraft ? "draft" : mappingId;

  useEffect(() => {
    if (mapping) {
      setEnvLabel(mapping.envLabel);
      setMappingType(mapping.mappingType);
      setSecretId(mapping.secretId ?? "");
      setTextValue(mapping.textValue ?? "");
      setProxyEnabled(mapping.proxyEnabled);
      setAllowedHosts(mapping.allowedHosts);
      setProxyToken(mapping.proxyPlaceholder);
      lastPersisted.current = formSnapshot(
        fieldsFromMapping(mapping),
        proxyBucketEnabled,
      );
    } else if (isDraft) {
      setEnvLabel("");
      setMappingType("secret");
      setSecretId("");
      setTextValue("");
      setProxyEnabled(false);
      setAllowedHosts([]);
      setProxyToken(null);
      lastPersisted.current = "";
    }
    setTokenRevealed(false);
  }, [hydrateKey, isDraft, proxyBucketEnabled]);

  const persist = useCallback(
    async (patch?: Partial<FormFields>) => {
      const fields: FormFields = { ...formRef.current, ...patch };
      if (!canPersist(fields)) return;

      const snapshot = formSnapshot(fields, proxyBucketEnabled);
      if (snapshot === lastPersisted.current) return;

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
        lastPersisted.current = snapshot;
        if (saved.proxyPlaceholder) {
          setProxyToken(saved.proxyPlaceholder);
        }
        onSavedRef.current(saved);
        notifyMappingUpdated(saved);
      } catch (e) {
        toast.fromError(e, "Failed to save mapping");
      } finally {
        setSaving(false);
      }
    },
    [bucketId, proxyBucketEnabled],
  );

  const commit = useCallback(
    (patch: Partial<FormFields>) => {
      if (patch.envLabel !== undefined) setEnvLabel(patch.envLabel);
      if (patch.mappingType !== undefined) setMappingType(patch.mappingType);
      if (patch.secretId !== undefined) setSecretId(patch.secretId);
      if (patch.textValue !== undefined) setTextValue(patch.textValue);
      if (patch.proxyEnabled !== undefined) setProxyEnabled(patch.proxyEnabled);
      if (patch.allowedHosts !== undefined) setAllowedHosts(patch.allowedHosts);

      const next = { ...formRef.current, ...patch };
      formRef.current = next;
      void persist(patch);
    },
    [persist],
  );

  const handleProxyToggle = useCallback(
    (enabled: boolean) => {
      setProxyEnabled(enabled);
      setTokenRevealed(false);

      if (!enabled) {
        setProxyToken(null);
        commit({ proxyEnabled: false });
        return;
      }

      const fields = { ...formRef.current, proxyEnabled: true };
      if (!canPersist(fields)) {
        const used = new Set(bucketProxyTokens);
        if (mapping?.proxyPlaceholder) used.delete(mapping.proxyPlaceholder);
        setProxyToken(generateProxyToken(used));
        return;
      }

      setSaving(true);
      void (async () => {
        try {
          const saved = await bridge.upsertBucketMapping({
            bucketId,
            envLabel: fields.envLabel.trim(),
            mappingType: fields.mappingType,
            secretId: fields.mappingType === "secret" ? fields.secretId : undefined,
            textValue: fields.mappingType === "text" ? fields.textValue.trim() : undefined,
            proxyEnabled: true,
            allowedHosts: fields.allowedHosts,
          });
          lastPersisted.current = formSnapshot(fields, proxyBucketEnabled);
          setProxyToken(saved.proxyPlaceholder ?? generateProxyToken());
          onSavedRef.current(saved);
          notifyMappingUpdated(saved);
        } catch (e) {
          setProxyEnabled(false);
          toast.fromError(e, "Failed to enable proxy token");
        } finally {
          setSaving(false);
        }
      })();
    },
    [bucketId, proxyBucketEnabled, commit, bucketProxyTokens, mapping?.proxyPlaceholder],
  );

  if (!mapping && !isDraft) {
    return (
      <div className="rounded-xl border border-border bg-surface p-6">
        <p className="text-sm text-text-muted">Select an env key to view or edit.</p>
      </div>
    );
  }

  const showProxyToken = proxyBucketEnabled && proxyEnabled;
  const maskedToken =
    proxyToken != null
      ? "•".repeat(Math.min(Math.max(proxyToken.length, 16), 40))
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
            onBlur={() => commit({ envLabel: formRef.current.envLabel })}
            placeholder="OPENAI_API_KEY"
          />
        </div>

        <div>
          <label className="text-xs font-medium text-text-muted">Type</label>
          <select
            className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
            value={mappingType}
            onChange={(e) => {
              const next = e.target.value as MappingType;
              commit({ mappingType: next });
            }}
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
                onChange={(id) => commit({ secretId: id })}
              />
            </div>
          ) : (
            <ArgusInput
              className="mt-1"
              value={textValue}
              onChange={(e) => setTextValue(e.target.value)}
              onBlur={() => commit({ textValue: formRef.current.textValue })}
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
                {proxyToken ? (
                  <div className="flex items-center gap-2 rounded-md border border-border bg-surface-raised/60 px-3 py-2">
                    <code className="block min-w-0 flex-1 break-all py-0.5 font-mono text-xs leading-snug text-text">
                      {tokenRevealed ? proxyToken : maskedToken}
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
                ) : null}

                <MappingAllowedHosts
                  hosts={allowedHosts}
                  onChange={(hosts) => commit({ allowedHosts: hosts })}
                  disabled={saving}
                />
              </div>
            )}
          </>
        )}

        {isDraft && onCancelDraft && (
          <div className="pt-2">
            <Button type="button" variant="secondary" onClick={onCancelDraft}>
              Cancel
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
