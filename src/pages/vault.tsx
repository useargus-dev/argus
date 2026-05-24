import { useCallback, useEffect, useMemo, useState } from "react";
import { Plus, ShieldCheck } from "lucide-react";

import { SecretFormDialog } from "../components/secrets/secret-form-dialog";
import { VaultLayout } from "../components/secrets/vault-layout";
import {
  VaultFiltersBar,
  type VaultFilterState,
} from "../components/secrets/vault-filters-bar";
import { Button } from "../components/ui/button";
import { filterSecrets } from "../lib/secret-utils";
import { toast } from "../lib/toast";
import { bridge, BridgeError } from "../lib/tauri-bridge";
import { useAuthStore } from "../state/auth-store";
import type { SecretDetail, SecretMeta, SecretWriteInput } from "../types/secret";

const defaultFilters: VaultFilterState = {
  query: "",
  types: [],
  environment: "all",
  tags: [],
};

export function VaultPage() {
  const scopes = useAuthStore((s) => s.scopes);
  const setScopes = useAuthStore((s) => s.setScopes);
  const appUnlocked = scopes?.app ?? false;

  const [secrets, setSecrets] = useState<SecretMeta[]>([]);
  const [filters, setFilters] = useState<VaultFilterState>(defaultFilters);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<SecretDetail | null>(null);
  const [listLoading, setListLoading] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [formOpen, setFormOpen] = useState(false);
  const [editing, setEditing] = useState<SecretDetail | null>(null);
  const [saving, setSaving] = useState(false);

  const filteredSecrets = useMemo(
    () => filterSecrets(secrets, filters),
    [secrets, filters],
  );

  const refreshScopes = useCallback(async () => {
    try {
      const s = await bridge.getScopeStatus();
      setScopes(s);
    } catch {
      /* handled by shell */
    }
  }, [setScopes]);

  const loadSecrets = useCallback(async () => {
    setListLoading(true);
    try {
      const list = await bridge.searchSecrets();
      setSecrets(list);
    } catch (e) {
      if (e instanceof BridgeError && e.code === "APP_LOCKED") return;
      toast.fromError(e, "Failed to load secrets");
    } finally {
      setListLoading(false);
    }
  }, []);

  const loadDetail = useCallback(async (id: string) => {
    setDetailLoading(true);
    try {
      const d = await bridge.getSecret(id);
      setDetail(d);
    } catch (e) {
      setDetail(null);
      toast.fromError(e, "Failed to load secret");
    } finally {
      setDetailLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshScopes();
    const id = window.setInterval(refreshScopes, 30_000);
    return () => window.clearInterval(id);
  }, [refreshScopes]);

  useEffect(() => {
    if (!appUnlocked) {
      setSecrets([]);
      setSelectedId(null);
      setDetail(null);
      return;
    }
    void loadSecrets();
  }, [appUnlocked, loadSecrets]);

  useEffect(() => {
    if (filteredSecrets.length === 0) {
      setSelectedId(null);
      setDetail(null);
      return;
    }
    if (!selectedId || !filteredSecrets.some((s) => s.id === selectedId)) {
      setSelectedId(filteredSecrets[0].id);
    }
  }, [filteredSecrets, selectedId]);

  useEffect(() => {
    if (!appUnlocked || !selectedId) {
      setDetail(null);
      return;
    }
    void loadDetail(selectedId);
  }, [appUnlocked, selectedId, loadDetail]);

  function openCreate() {
    setEditing(null);
    setFormOpen(true);
  }

  function openEdit() {
    if (!detail) return;
    setEditing(detail);
    setFormOpen(true);
  }

  async function handleDelete() {
    if (!selectedId) return;
    if (!confirm("Delete this secret permanently?")) return;
    try {
      await bridge.deleteSecret(selectedId);
      toast.success("Secret deleted");
      setSelectedId(null);
      setDetail(null);
      await loadSecrets();
    } catch (e) {
      toast.fromError(e, "Delete failed");
    }
  }

  async function handleSave(input: SecretWriteInput) {
    setSaving(true);
    try {
      if (editing) {
        const editId = editing.id;
        await bridge.updateSecret(editId, input);
        toast.success("Secret updated");
        setFormOpen(false);
        setEditing(null);
        await loadSecrets();
        await loadDetail(editId);
      } else {
        const created = await bridge.createSecret(input);
        toast.success("Secret added");
        setFormOpen(false);
        setEditing(null);
        await loadSecrets();
        setSelectedId(created.id);
        await loadDetail(created.id);
      }
    } catch (e) {
      toast.fromError(e, "Save failed");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="mx-auto max-w-[1400px] px-2 py-2">
      <div className="mb-6 flex items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight text-text">Vault</h1>
          <p className="mt-1 text-sm text-text-muted">
            All secrets are encrypted with your master key.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span
            className={
              appUnlocked
                ? "inline-flex items-center gap-1.5 rounded-md border border-success-border bg-success-muted px-2.5 py-1 text-xs text-success"
                : "inline-flex items-center gap-1.5 rounded-md border border-border bg-surface px-2.5 py-1 text-xs text-text-muted"
            }
          >
            <ShieldCheck className="size-3.5" aria-hidden />
            {appUnlocked ? "Unlocked" : "Locked"}
          </span>
          {appUnlocked && (
            <Button
              type="button"
              variant="primary"
              className="h-10 gap-2 text-sm"
              onClick={openCreate}
            >
              <Plus className="size-4" aria-hidden />
              Add secret
            </Button>
          )}
        </div>
      </div>

      {!appUnlocked ? (
        <p className="text-sm text-text-muted">
          Unlock the app to view and manage secrets.
        </p>
      ) : (
        <>
          <VaultFiltersBar secrets={secrets} filters={filters} onChange={setFilters} />

          {listLoading ? (
            <p className="text-sm text-text-muted">Loading secrets…</p>
          ) : (
            <VaultLayout
              secrets={filteredSecrets}
              selectedId={selectedId}
              onSelect={setSelectedId}
              detail={detail}
              detailLoading={detailLoading}
              onEdit={openEdit}
              onDelete={handleDelete}
            />
          )}
        </>
      )}

      <SecretFormDialog
        open={formOpen}
        initial={editing}
        saving={saving}
        onClose={() => {
          setFormOpen(false);
          setEditing(null);
        }}
        onSave={handleSave}
      />
    </div>
  );
}
