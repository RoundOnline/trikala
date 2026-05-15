# Changelog

All notable changes to `trikala` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and trikala adopts [SemVer](https://semver.org/) from v1.0 onward
(pre-1.0 minor bumps may be breaking — see axiom T3 / U13).

## [Unreleased]

Tracked work for the next alpha:

- `trikala new` wired against `cargo-generate`
- `trikala dev` hot-reload loop (notify + dexterous_developer)
- `trikala build` cross-platform via `cargo-mobile2` for Android
- `trikala deploy cloudflare` via `wrangler`
- 6 more templates: 2d-platformer, 2d-topdown, 2d-puzzle, board-game,
  card-game, 3d-arena

## [0.1.0-alpha.1] — 2026-05-15

Initial scaffold. Public surface frozen by axiom v1; implementations
arrive incrementally in subsequent alphas.

### Added

- `trikala-core` foundation crate (`Phase`, `TrikalaError`, `ProjectConfig`)
- `trikala` CLI binary with 8 commands stubbed (`new`, `dev`, `build`,
  `deploy`, `claim`, `doctor`, `use`, `upgrade`); `--help` complete
- `templates/blank` — 148-line standalone wgpu + winit window
- `AGENT.md` (270 lines) — canonical AI agent instruction set,
  mirrored to `.claude/skills/trikala/SKILL.md` and `.cursorrules`
- 83 axioms locked across T (Tenets), U (UX/CLI), F (Foundation),
  D (Deploy), C (Community), I (Integration) with v2 trigger gate
- Design documents: architecture, pitch, phases, quickstart,
  AI prompt cards, hosting policy, telemetry schema
- CI matrix: fmt + clippy + test on Win/Mac/Linux + wasm32 build
- Dual MIT / Apache-2.0 license

### Not yet shipping

Each stub returns `anyhow::bail!("... coming in 0.1.0-alpha.2")`:

- Real scaffold flow (`trikala new` currently bails)
- Hot reload loop (`trikala dev` currently bails)
- Cross-compile build (`trikala build` currently bails)
- Deploy targets (`trikala deploy` currently bails)
- Self-update (`trikala upgrade` currently bails)
- `trikala-server` (round.online hosting) — separate repository,
  arrives in v0.2

[Unreleased]: https://github.com/RoundOnline/trikala/compare/v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/RoundOnline/trikala/releases/tag/v0.1.0-alpha.1
