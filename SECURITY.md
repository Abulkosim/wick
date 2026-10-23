# Security

## Supported versions

Only the [latest release](https://github.com/Abulkosim/wick/releases/latest) receives security fixes. Update before reporting, if you can.

## Report a vulnerability

Report privately through [GitHub private vulnerability reporting](https://github.com/Abulkosim/wick/security/advisories/new).

If that is disabled, open an [issue](https://github.com/Abulkosim/wick/issues/new) titled `Security` with no details. The maintainer will reach out.

Include:

- Wick version and macOS version.
- What an attacker can do, and what they need first.
- Steps to reproduce.

## What to expect

Wick is an open-source project maintained by one person. Responses are best effort. There is no bug bounty.

Confirmed issues are fixed in a new release. The advisory is published after the fix ships, with credit if you want it.

## Automated checks

Every push to `main`, every pull request, and a weekly schedule run `cargo audit`, `cargo deny` (advisories, licenses, bans, sources; see `src-tauri/deny.toml`), and `pnpm audit --prod --audit-level high`. OpenSSF Scorecard runs on `main` and weekly. Release builds carry signed build provenance and SBOM attestations; verify them with `gh attestation verify <file> --repo Abulkosim/wick`.

## Scope

In scope:

- Wick changing a system setting (`pmset`, energy preferences) or leaving a sleep assertion behind after it quits.
- Network requests to hosts other than those in [PRIVACY.md](PRIVACY.md#network-requests).
- Update checks or installs that accept an unsigned, altered, or mismatched-version release, or a download from an unexpected address.
- Code execution or crashes from a crafted settings file, process name, or power assertion.
- Reading files beyond those listed in [PRIVACY.md](PRIVACY.md#what-wick-reads).

Out of scope:

- The missing Apple Developer ID signature and notarization. Known; see [Install](README.md#install).
- Attacks that need an already compromised user account or administrator access.
- Missing hardening without a concrete attack.
