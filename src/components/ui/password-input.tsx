import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";

import { ArgusInput } from "./argus-input";
import { Field } from "./field";

interface PasswordInputProps {
  label?: string;
  value: string;
  onChange: (v: string) => void;
  error?: string;
  autoComplete?: string;
  placeholder?: string;
}

export function PasswordInput({
  label = "Password",
  value,
  onChange,
  error,
  autoComplete,
  placeholder,
}: PasswordInputProps) {
  const [show, setShow] = useState(false);

  return (
    <Field label={label} error={error}>
      <div className="relative">
        <ArgusInput
          type={show ? "text" : "password"}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          autoComplete={autoComplete}
          placeholder={placeholder}
          spellCheck={false}
          className="argus-password-input pr-10"
        />
        <button
          type="button"
          className="absolute inset-y-0 right-0 flex w-10 items-center justify-center text-text-muted transition-colors hover:text-text"
          onClick={() => setShow((s) => !s)}
          tabIndex={-1}
          aria-label={show ? "Hide password" : "Show password"}
        >
          {show ? <EyeOff size={16} aria-hidden /> : <Eye size={16} aria-hidden />}
        </button>
      </div>
    </Field>
  );
}
