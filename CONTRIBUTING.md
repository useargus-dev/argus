# Contributing to Argus

Thank you for your interest in Argus. This project is early-stage; contributions are welcome with the expectations below.

## Before you start

1. Read [README.md](README.md) and [docs/architecture.md](docs/architecture.md).
2. Understand the [LICENSE](LICENSE) (AGPL-3.0): contributions will be under the same license.
3. Do not commit secrets, `~/.argus` databases, tokens, or personal `.env` files.

## Development setup

```bash
pnpm install
pnpm tauri dev
```

See [docs/build-deps.md](docs/build-deps.md) for SQLCipher on Windows.

## Pull requests

1. Fork the repository and create a branch from `main` (do not push directly to `main` once branch protection is enabled).
2. Keep changes focused; match existing code style.
3. Run `pnpm exec tsc --noEmit` and `pnpm lint` before submitting.
4. Open a PR into `main`. Wait for the **CI / lint** check to pass before merging.
5. Describe **what** and **why** in the PR. Link issues if applicable.
6. For security-sensitive changes, explain threat/impact in the PR body.

### Protect `main` on GitHub (maintainers, one-time)

Branch protection is configured in GitHub, not in Actions. In the repo on github.com:

**Settings → Rules → Rulesets → New branch ruleset** (or **Settings → Branches → Add rule** for classic protection):

- Target: `main` (or default branch)
- **Require a pull request before merging**
- **Block force pushes**
- Optional: **Require status checks** → add **CI / lint** (appears after CI has run on at least one PR)
- Optional: **Require approvals** (use 0 if you are the only maintainer and merge your own PRs)

Save. After that, direct pushes to `main` are rejected; only merged PRs update `main`.

## License

By contributing, you agree that your contributions are licensed under the **GNU Affero General Public License v3.0** (or later), and you have the right to license them.

If you cannot license your work under AGPL-3.0, do not submit a pull request; contact maintainers about alternative arrangements.

## What we’re looking for

- Bug fixes and tests (when a test harness exists)
- Documentation improvements
- Accessibility and UI polish
- Rust/Tauri hardening aligned with [docs/security.md](docs/security.md)

Large features (IPC server, client SDKs) should be discussed in an issue first—see [docs/plan.md](docs/plan.md).

## Code of conduct

Be respectful and constructive. Maintainers may close issues or PRs that are abusive, off-topic, or outside project scope.
