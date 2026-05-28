## What

<!-- What changed? Be specific (screens, commands, modules). -->

## Why

<!-- Why is this change needed? Link an issue if applicable: Fixes # -->

## Security impact

<!-- For security-sensitive code (auth, crypto, IPC, SQLCipher, grants): -->
<!-- - Threat considered -->
<!-- - How this mitigates or documents risk -->
<!-- - Out of scope / accepted risk (if any) -->
<!-- For non-security changes, write "None" or "N/A". -->

## How to test

<!-- Steps a reviewer can follow. Example: -->
<!-- 1. pnpm install && pnpm tauri dev -->
<!-- 2. ... -->

## Checklist

- [ ] I ran `pnpm exec tsc --noEmit` and `pnpm lint` locally (or CI is green)
- [ ] I did not commit secrets, tokens, or `~/.argus` database files
- [ ] I read [CONTRIBUTING.md](../CONTRIBUTING.md) and [docs/security.md](../docs/security.md) if touching security-related code
