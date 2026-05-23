import { Eclipse } from "lucide-react";

export function RegisterBrand() {
  return (
    <div className="mb-6 flex items-center justify-center gap-2">
      <div
        className="grid size-9 place-items-center rounded-md border bg-[var(--argus-brand-bg)]"
        style={{ borderColor: "var(--argus-brand-border)" }}
      >
        <Eclipse
          className="size-5"
          style={{ color: "var(--argus-brand-icon)" }}
          aria-hidden
        />
      </div>
      <span className="text-lg font-semibold tracking-tight text-text">Argus</span>
    </div>
  );
}
