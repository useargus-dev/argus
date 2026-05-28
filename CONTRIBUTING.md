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

`main` is protected: **no direct pushes**. Open a branch, open a PR, get at least one approval, and wait for the **CI / lint** check to pass before merging.

1. Fork the repository and create a branch from `main`.
2. Keep changes focused; match existing code style.
3. Run `pnpm exec tsc --noEmit` and `pnpm lint` before submitting.
4. Describe **what** and **why** in the PR. Link issues if applicable.
5. For security-sensitive changes, explain threat/impact in the PR body.

### Branch protection (maintainers)

Protection is defined in [`.github/rulesets/protect-main.json`](.github/rulesets/protect-main.json) and applied with the [**Sync branch protection**](.github/workflows/sync-branch-protection.yml) workflow (`workflow_dispatch`).

After the first merge of these files, enable protection either way:

1. **Workflow:** add secret `REPO_ADMIN_TOKEN` (PAT with **Administration** read/write on this repo), then run **Actions → Sync branch protection**.
2. **Manual:** **Settings → Rules → Rulesets → Import a ruleset** and choose `.github/rulesets/protect-main.json`.

Both create the same ruleset: PR-only updates to `main`, 1 required review, no force-push/delete, and required **CI / lint**.

To change rules (e.g. review count), edit `protect-main.json` and re-run the sync workflow. Solo maintainers who cannot get a second reviewer may lower `required_approving_review_count` to `0` while keeping the PR-only rule.

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
