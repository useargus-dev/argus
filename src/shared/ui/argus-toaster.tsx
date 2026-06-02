import { Toaster as SonnerToaster } from "sonner";

import { useThemeStore } from "@/state/theme-store";

export function ArgusToaster() {
  const theme = useThemeStore((s) => s.theme);

  return (
    <SonnerToaster
      theme={theme}
      position="bottom-right"
      closeButton
      toastOptions={{
        classNames: {
          toast: "argus-toast",
          title: "argus-toast-title",
          description: "argus-toast-description",
          success: "argus-toast-success",
          info: "argus-toast-info",
          error: "argus-toast-error",
          closeButton: "argus-toast-close",
        },
      }}
    />
  );
}
