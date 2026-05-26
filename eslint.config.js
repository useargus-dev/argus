import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import jsxA11y from "eslint-plugin-jsx-a11y";

export default tseslint.config(
  { ignores: ["dist/", "src-tauri/"] },

  js.configs.recommended,
  ...tseslint.configs.recommended,

  {
    plugins: {
      "react-hooks": reactHooks,
      "jsx-a11y": jsxA11y,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      ...jsxA11y.configs.recommended.rules,

      // Tauri bridge calls in effects legitimately set state from async callbacks
      "react-hooks/set-state-in-effect": "off",

      // Desktop app — autoFocus on modals and primary inputs is intentional
      "jsx-a11y/no-autofocus": "off",

      // Label component receives htmlFor via props; static analysis can't verify
      "jsx-a11y/label-has-associated-control": [
        "warn",
        {
          assert: "either",
          depth: 3,
          controlComponents: ["ArgusInput", "SecretPicker", "Textarea", "PasswordInput"],
        },
      ],

      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
);
