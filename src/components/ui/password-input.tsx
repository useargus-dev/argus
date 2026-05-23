import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";

import { cn } from "../../lib/cn";
import { Input } from "./input";

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
    <div className="relative">
      <Input
        label={label}
        type={show ? "text" : "password"}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        error={error}
        autoComplete={autoComplete}
        placeholder={placeholder}
        className="pr-10"
      />
      <button
        type="button"
        className={cn(
          "absolute right-2 top-8 text-text-muted hover:text-text",
        )}
        onClick={() => setShow((s) => !s)}
        tabIndex={-1}
        aria-label={show ? "Hide password" : "Show password"}
      >
        {show ? <EyeOff size={16} /> : <Eye size={16} />}
      </button>
    </div>
  );
}
