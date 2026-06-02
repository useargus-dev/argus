import { ArgusInput } from "@/shared/ui/argus-input";
import {
  formatRecoveryCode,
  normalizeRecoveryCode,
} from "@/shared/utils/recovery-code";

interface RecoveryCodeInputProps {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

export function RecoveryCodeInput({ value, onChange, disabled }: RecoveryCodeInputProps) {
  const normalized = normalizeRecoveryCode(value);
  const display =
    normalized.length > 4
      ? formatRecoveryCode(normalized)
      : normalized;

  function handleChange(raw: string) {
    onChange(normalizeRecoveryCode(raw));
  }

  return (
    <ArgusInput
      inputMode="text"
      autoComplete="off"
      autoCorrect="off"
      spellCheck={false}
      maxLength={9}
      value={display}
      onChange={(e) => handleChange(e.target.value)}
      onPaste={(e) => {
        e.preventDefault();
        handleChange(e.clipboardData.getData("text"));
      }}
      disabled={disabled}
      placeholder="XXXXXXXX"
      className="text-center font-mono text-lg tracking-[0.25em] uppercase"
      aria-label="Recovery code"
    />
  );
}
