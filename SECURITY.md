# Security policy

## Supported versions

| Version | Status |
|---|---|
| `0.1.x` (alpha) | actively patched |
| Older alphas | not supported |

There is no LTS until v1.0.

## Reporting a vulnerability

**Do not file public issues for security vulnerabilities.**

Use one of these channels:

1. **GitHub Security Advisory** (preferred) — open a draft advisory
   on this repository at `Security → Advisories → Report a
   vulnerability`. Coordinated disclosure is handled there.

2. **Email** — `security@round.online`. Encrypt sensitive details
   with our PGP key if needed (key published at
   `https://round.online/.well-known/pgp-key.asc` once trikala
   goes public; until then, send plaintext and we will rotate any
   exposed credentials).

## What to expect

- **Acknowledgement**: within 48 hours
- **Initial assessment**: within 5 business days
- **Fix or mitigation plan**: depends on severity (CVSS-aligned)
- **Coordinated disclosure**: we agree on a date with you before
  public disclosure

## Scope

In scope:
- The `trikala` CLI and foundation crates in this repository
- The blank template's runtime behaviour
- AGENT.md / `.claude/skills/` instruction integrity (axiom F32)

Out of scope (report to the relevant upstream):
- Vulnerabilities in `wgpu`, `winit`, `kira`, `egui`, `cargo-generate`
  — these are pinned ecosystem dependencies (axiom F21)
- Vulnerabilities in the `round.online/play/*` hosting backend —
  that backend lives in a separate private repository

## What we will not do

- Publicly name a reporter without their consent
- Threaten legal action against good-faith researchers operating
  within this policy
- Pay bounties at this stage (we're a pre-release alpha — please
  report anyway; we credit researchers in the changelog)

Thank you for taking the time to report responsibly.
