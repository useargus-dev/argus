# Security Policy

Argus is a **local secrets vault**. Treat it as security-sensitive software even during early development.

## Supported versions

| Version | Supported |
|---------|-----------|
| `0.1.x` (main) | Yes — best effort |

## Reporting a vulnerability

**Please do not** open public GitHub issues for security bugs.

1. Use **[GitHub Security Advisories](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)** → *Report a vulnerability* on this repository.
2. Include steps to reproduce, impact, and affected version/commit if known.

We aim to acknowledge reports within **7 days**. Fixes and coordinated disclosure timelines depend on severity.

## Scope

**In scope**

- Argus desktop app (Tauri + Rust core + WebView UI)
- SQLCipher database handling, authentication, secret/bucket commands
- Local data under `~/.argus/`

**Out of scope (for now)**

- Third-party dependency vulnerabilities (report upstream; we will upgrade)
- Issues in apps that consume secrets **after** Argus injects them
- Attacks requiring full OS/kernel compromise (documented in [docs/security.md](docs/security.md))

## Full specification

See [docs/security.md](docs/security.md) for threat model, crypto parameters, and release checklist.

## Safe harbor

We appreciate responsible disclosure. Researchers who follow this policy and give us reasonable time to fix issues before public disclosure will not be pursued for good-faith research.
