import { AppLogo } from "@/shared/ui/app-logo";

export function AppBrand() {
  return (
    <div className="mb-6 flex items-center justify-center gap-2">
      <div
        className="grid size-9 place-items-center rounded-md border"
        style={{
          backgroundColor: "var(--brand-bg)",
          borderColor: "var(--brand-border)",
        }}
      >
        <AppLogo
          size={20}
          className="text-brand-icon"
          style={{ color: "var(--brand-icon)" }}
        />
      </div>
      <span className="text-lg font-semibold tracking-tight text-text">Argus</span>
    </div>
  );
}
