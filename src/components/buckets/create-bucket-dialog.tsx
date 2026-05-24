import { useEffect, useState } from "react";
import { X } from "lucide-react";

import type { CreateBucketInput } from "../../types/bucket";
import { ArgusInput } from "../ui/argus-input";
import { Button } from "../ui/button";
import { Field } from "../ui/field";
import { Textarea } from "../ui/textarea";

interface CreateBucketDialogProps {
  open: boolean;
  saving: boolean;
  onClose: () => void;
  onCreate: (input: CreateBucketInput) => void;
}

export function CreateBucketDialog({
  open,
  saving,
  onClose,
  onCreate,
}: CreateBucketDialogProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");

  useEffect(() => {
    if (!open) return;
    setName("");
    setDescription("");
  }, [open]);

  if (!open) return null;

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    onCreate({
      name: name.trim(),
      description: description.trim() || undefined,
    });
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-bg/80 p-4 backdrop-blur-sm"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-bucket-title"
    >
      <div className="w-full max-w-md rounded-xl border border-border bg-surface p-6 shadow-lg">
        <div className="flex items-start justify-between gap-4">
          <h2 id="create-bucket-title" className="text-lg font-semibold text-text">
            Create bucket
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
              placeholder="Acme Backend"
            />
          </Field>
          <Field label="Description (optional)">
            <Textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={2}
              placeholder="FastAPI service, production environment"
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
              disabled={saving || !name.trim()}
            >
              {saving ? "Creating…" : "Create bucket"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
