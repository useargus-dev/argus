import { cn } from "@/core/cn";

export function RegisterProgress({ step }: { step: 1 | 2 | 3 }) {
  return (
    <div className="mb-4 flex items-center gap-1.5">
      {([1, 2, 3] as const).map((n) => (
        <div
          key={n}
          className={cn(
            "h-1 flex-1 rounded-full transition-colors",
            n <= step ? "bg-primary" : "bg-border",
          )}
        />
      ))}
    </div>
  );
}
