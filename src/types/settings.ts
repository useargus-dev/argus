import type { SecondFactorType } from "./auth";

export interface SecondFactorStatus {
  activeSecondFactor: SecondFactorType;
  totpEnrolled: boolean;
  biometricEnrolled: boolean;
}

export const AUTO_LOCK_OPTIONS = [
  { value: "5", label: "5 minutes" },
  { value: "15", label: "15 minutes" },
  { value: "30", label: "30 minutes" },
  { value: "60", label: "1 hour" },
] as const;

export const EXPIRY_NOTIFY_OPTIONS = [
  { value: "3", label: "3 days" },
  { value: "7", label: "7 days" },
  { value: "30", label: "30 days" },
] as const;
