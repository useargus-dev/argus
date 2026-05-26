import { useCallback, useEffect, useState } from "react";
import { Check, Plus, Trash2 } from "lucide-react";

import { bridge } from "../../lib/tauri-bridge";
import { toast } from "../../lib/toast";
import type { BucketMapping } from "../../types/bucket";
import type { SecretMeta } from "../../types/secret";
import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { SecretPicker } from "./secret-picker";

interface DraftRow {
  key: string;
  envLabel: string;
  secretId: string;
}

interface BucketMappingsPanelProps {
  bucketId: string;
  onMappingsChange?: () => void;
}

export function BucketMappingsPanel({
  bucketId,
  onMappingsChange,
}: BucketMappingsPanelProps) {
  const [mappings, setMappings] = useState<BucketMapping[]>([]);
  const [secrets, setSecrets] = useState<SecretMeta[]>([]);
  const [drafts, setDrafts] = useState<DraftRow[]>([]);
  const [edits, setEdits] = useState<Record<string, { envLabel: string; secretId: string }>>(
    {},
  );
  const [loading, setLoading] = useState(true);
  const [savingKey, setSavingKey] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [mapList, secretList] = await Promise.all([
        bridge.listBucketMappings(bucketId),
        bridge.searchSecrets(),
      ]);
      setMappings(mapList);
      setSecrets(secretList);
      setEdits({});
    } catch (e) {
      toast.fromError(e, "Failed to load mappings");
    } finally {
      setLoading(false);
    }
  }, [bucketId]);

  useEffect(() => {
    void load();
  }, [load]);

  function addDraftRow() {
    setDrafts((prev) => [
      ...prev,
      { key: crypto.randomUUID(), envLabel: "", secretId: "" },
    ]);
  }

  function updateDraft(key: string, patch: Partial<DraftRow>) {
    setDrafts((prev) =>
      prev.map((d) => (d.key === key ? { ...d, ...patch } : d)),
    );
  }

  function removeDraft(key: string) {
    setDrafts((prev) => prev.filter((d) => d.key !== key));
  }

  function getEdit(mapping: BucketMapping) {
    return (
      edits[mapping.id] ?? {
        envLabel: mapping.envLabel,
        secretId: mapping.secretId,
      }
    );
  }

  function setEdit(
    mappingId: string,
    patch: Partial<{ envLabel: string; secretId: string }>,
  ) {
    setEdits((prev) => {
      const base =
        prev[mappingId] ??
        (() => {
          const m = mappings.find((x) => x.id === mappingId);
          return m
            ? { envLabel: m.envLabel, secretId: m.secretId }
            : { envLabel: "", secretId: "" };
        })();
      return { ...prev, [mappingId]: { ...base, ...patch } };
    });
  }

  async function saveMapping(
    saveKey: string,
    envLabel: string,
    secretId: string,
    onDone?: () => void,
  ) {
    if (!envLabel.trim()) {
      toast.error("Env name is required");
      return;
    }
    if (!secretId) {
      toast.error("Select a vault secret");
      return;
    }
    setSavingKey(saveKey);
    try {
      await bridge.upsertBucketMapping({
        bucketId,
        envLabel: envLabel.trim(),
        secretId,
      });
      toast.success("Mapping saved");
      onDone?.();
      onMappingsChange?.();
      await load();
    } catch (e) {
      toast.fromError(e, "Failed to save mapping");
    } finally {
      setSavingKey(null);
    }
  }

  async function deleteMapping(id: string) {
    if (!confirm("Remove this mapping?")) return;
    setSavingKey(id);
    try {
      await bridge.deleteBucketMapping(id);
      toast.success("Mapping removed");
      onMappingsChange?.();
      await load();
    } catch (e) {
      toast.fromError(e, "Failed to remove mapping");
    } finally {
      setSavingKey(null);
    }
  }

  return (
    <section className="mt-6 rounded-xl border border-border bg-surface p-5">
      <div className="mb-4 flex flex-wrap items-end justify-between gap-3">
        <div>
          <h2 className="text-sm font-semibold text-text">Mappings</h2>
          <p className="mt-1 text-xs text-text-muted">
            Link env variable names to secrets from your vault.
          </p>
        </div>
        <Button
          type="button"
          variant="secondary"
          className="h-9 gap-1.5 text-sm"
          onClick={addDraftRow}
        >
          <Plus className="size-4" aria-hidden />
          Add mapping
        </Button>
      </div>

      {loading ? (
        <p className="text-sm text-text-muted">Loading mappings…</p>
      ) : secrets.length === 0 ? (
        <p className="text-sm text-text-muted">
          Add secrets in the vault before creating mappings.
        </p>
      ) : (
        <>
          <div className="mb-2 hidden gap-3 text-[10px] font-semibold uppercase tracking-wider text-text-muted sm:grid sm:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_auto]">
            <span>Env name</span>
            <span>Vault secret</span>
            <span className="sr-only">Actions</span>
          </div>

          <div className="space-y-2">
            {mappings.map((mapping) => {
              const edit = getEdit(mapping);
              const dirty =
                edit.envLabel !== mapping.envLabel ||
                edit.secretId !== mapping.secretId;
              const rowKey = mapping.id;

              return (
                <div
                  key={mapping.id}
                  className="grid gap-2 rounded-lg border border-border bg-surface-raised/40 p-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_auto] sm:items-center sm:gap-3"
                >
                  <label className="min-w-0">
                    <span className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-text-muted sm:sr-only">
                      Env name
                    </span>
                    <ArgusInput
                      value={edit.envLabel}
                      onChange={(e) =>
                        setEdit(mapping.id, { envLabel: e.target.value })
                      }
                      placeholder="DATABASE_URL"
                      className="font-mono text-xs uppercase"
                      disabled={savingKey === rowKey}
                    />
                  </label>
                  <label className="min-w-0">
                    <span className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-text-muted sm:sr-only">
                      Vault secret
                    </span>
                    <SecretPicker
                      secrets={secrets}
                      value={edit.secretId}
                      onChange={(secretId) =>
                        setEdit(mapping.id, { secretId })
                      }
                      disabled={savingKey === rowKey}
                    />
                  </label>
                  <div className="flex justify-end gap-1">
                    {dirty && (
                      <button
                        type="button"
                        title="Save mapping"
                        aria-label="Save mapping"
                        disabled={savingKey === rowKey}
                        onClick={() =>
                          saveMapping(rowKey, edit.envLabel, edit.secretId)
                        }
                        className="grid size-8 place-items-center rounded-md text-success hover:bg-success-muted"
                      >
                        <Check className="size-4" aria-hidden />
                      </button>
                    )}
                    <button
                      type="button"
                      title="Delete mapping"
                      aria-label="Delete mapping"
                      disabled={savingKey === rowKey}
                      onClick={() => deleteMapping(mapping.id)}
                      className="grid size-8 place-items-center rounded-md text-text-muted hover:bg-danger/10 hover:text-danger"
                    >
                      <Trash2 className="size-4" aria-hidden />
                    </button>
                  </div>
                </div>
              );
            })}

            {drafts.map((draft) => (
              <div
                key={draft.key}
                className="grid gap-2 rounded-lg border border-dashed border-border bg-surface p-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)_auto] sm:items-center sm:gap-3"
              >
                <label className="min-w-0">
                  <span className="mb-1 block text-[10px] font-semibold uppercase tracking-wider text-text-muted sm:sr-only">
                    Env name
                  </span>
                  <ArgusInput
                    value={draft.envLabel}
                    onChange={(e) =>
                      updateDraft(draft.key, { envLabel: e.target.value })
                    }
                    placeholder="DATABASE_URL"
                    className="font-mono text-xs uppercase"
                    autoFocus
                    disabled={savingKey === draft.key}
                  />
                </label>
                <div className="flex min-w-0">
                  <SecretPicker
                    secrets={secrets}
                    value={draft.secretId}
                    onChange={(secretId) =>
                      updateDraft(draft.key, { secretId })
                    }
                    disabled={savingKey === draft.key}
                  />
                </div>
                <div className="flex justify-end gap-1">
                  <button
                    type="button"
                    title="Save mapping"
                    aria-label="Save mapping"
                    disabled={savingKey === draft.key}
                    onClick={() =>
                      saveMapping(
                        draft.key,
                        draft.envLabel,
                        draft.secretId,
                        () => removeDraft(draft.key),
                      )
                    }
                    className="grid size-8 place-items-center rounded-md text-success hover:bg-success-muted"
                  >
                    <Check className="size-4" aria-hidden />
                  </button>
                  <button
                    type="button"
                    title="Cancel"
                    aria-label="Cancel"
                    onClick={() => removeDraft(draft.key)}
                    className="grid size-8 place-items-center rounded-md text-text-muted hover:bg-surface-raised"
                  >
                    <Trash2 className="size-4" aria-hidden />
                  </button>
                </div>
              </div>
            ))}
          </div>

          {mappings.length === 0 && drafts.length === 0 && (
            <p className="mt-4 text-center text-sm text-text-muted">
              No mappings yet. Click Add mapping to link env names to vault
              secrets.
            </p>
          )}
        </>
      )}
    </section>
  );
}
