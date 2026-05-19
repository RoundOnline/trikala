# Changelog

All notable changes to `trikala` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and trikala adopts [SemVer](https://semver.org/) from v1.0 onward
(pre-1.0 minor bumps may be breaking — see axiom T3 / U13).

## [Unreleased]

Tracked work for alpha.3:

- `trikala deploy` + `trikala claim` (depends on `trikala-server`)
- `trikala-server` initial deploy of the Workers + R2 + Pages backend
- 6 more templates: 2d-platformer, 2d-topdown, 2d-puzzle, board-game,
  card-game, 3d-arena
- `examples/showroom` — full landscape + 3D + font + HUD demo
- Shader hot-reload inside `trikala dev` (`.wgsl` changes update
  in-process without rebuilding)
- `trikala doctor` — real toolchain + GPU + network checks

## [0.1.0-alpha.2] — TBD

The inner loop now works: scaffold a project, iterate with a watcher,
build a release artifact. Deploy is still server-pending.

### Added

- `trikala new <name>` wired — embeds `templates/blank` into the CLI
  binary at compile time, scaffolds offline, no network call
- `trikala dev` wired — wraps `cargo run`, watches `src/` + `shaders/`,
  restarts the child on `.rs` change. `--no-watch` for CI smoke tests.
- `trikala build` wired — wraps `cargo build --release`, copies the
  binary plus `art/ music/ sfx/ fonts/ shaders/ levels/` into `dist/`
- `templates/blank` enhanced — asset-folder structure
  (`art/ music/ sfx/ fonts/`) with READMEs documenting format and
  hot-reload behavior
- `install.sh` + `install.ps1` — one-line CLI install on
  macOS / Linux / Windows from GitHub Releases
- `examples/mood` — multi-room portal prototype (reference for the
  scale of project trikala targets)
- Release pipeline in `RoundOnline/trikala-machinery` —
  GitHub Actions workflow builds 4 platform targets on `v*` tag push
  and uploads to this repo's GitHub Releases

### Changed

- CLI source moved out of this repo into the private
  `RoundOnline/trikala-machinery` repo. This public repo now contains
  only what users fork: templates, examples, the `trikala-core`
  foundation crate, and design docs. The CLI binary ships through
  install.sh / GitHub Releases.
- Deploy model: `trikala deploy` will be **server-mediated** (CLI
  uploads to our endpoint, server handles hosting). No more BYO
  Cloudflare / wrangler flow. The architecture.md sections about
  `CF_API_TOKEN` and BYO Cloudflare are obsoleted by this change.

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
- Dual MIT / Apache-2.0 license

[Unreleased]: https://github.com/RoundOnline/trikala/compare/v0.1.0-alpha.2...HEAD
[0.1.0-alpha.2]: https://github.com/RoundOnline/trikala/compare/v0.1.0-alpha.1...v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/RoundOnline/trikala/releases/tag/v0.1.0-alpha.1
