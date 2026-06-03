import { toast as sonner } from "sonner";

const base = {
  position: "bottom-right" as const,
  closeButton: true,
};

function messageFromError(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error) return error;
  return fallback;
}

export const toast = {
  success(message: string, description?: string) {
    sonner.success(message, { ...base, description });
  },

  info(message: string, description?: string) {
    sonner.info(message, { ...base, description });
  },

  error(message: string, description?: string) {
    sonner.error(message, { ...base, description });
  },

  fromError(error: unknown, fallback: string) {
    toast.error(messageFromError(error, fallback));
  },
};
