# trikala

> **Copy a game. Make it yours. Ship it in your browser.**
>
> Born, built, shipped — three commands, no signup.
> The first Rust game framework that ships with a Claude Skill.

```
trikala new starfighter --template 2d-platformer
   # → 200 lines of plain Rust in your folder. No engine to learn.

trikala dev
   # → edit, see it live. Press F1 for the tuning panel.

trikala deploy
   # → https://round.online/play/anon/starfighter
   #   (Valid 7 days. `trikala claim` to keep it.)
```

That's it. No account. No project linking. No "first set up your build
target." Sixty seconds per iteration after the first build.

---

## What it is

A curated registry of **standalone, readable, fork-able game templates**
in Rust + wgpu — plus a CLI that scaffolds, hot-reloads, and ships them.

Templates are the product. The CLI is delivery. Hosting closes the loop.

There is **no engine**. Each template is a self-contained 100–300 lines
of Rust that uses `wgpu`, `winit`, `kira`, and `egui` directly. You can
delete `trikala` tomorrow and your game still compiles.

## Why this shape

The Rust gamedev ecosystem already has its engine (Bevy). What it
doesn't have is a way for a beginner — or an AI coding agent — to
go from blank screen to deployed game without first learning an ECS,
a scene graph, or a plugin system.

Templates that are short and dependency-honest solve both:

| Audience | Why short templates win |
|---|---|
| **Beginners** | Read 200 lines, understand the whole game. No abstraction to learn. |
| **AI agents** (Cline, Aider, Cursor, Claude Code) | One context window holds the whole codebase. The agent edits the right line because there's no hidden behavior. |
| **Veterans** | Fork, own, modify, ship. No lock-in. No "trikala 0.4 broke my game." |

This is the [shadcn/ui](https://ui.shadcn.com) pattern, applied to game
development.

## AI-native by design — not by accident

trikala ships an `AGENT.md` at the repo root and mirrors it into the
formats used by every major AI coding agent:

- **Claude Code** — reads `.claude/skills/trikala/SKILL.md`
- **Cursor** — reads `.cursorrules`
- **Aider / Cline / OpenHands / Copilot** — read `AGENT.md` directly

Drop a trikala template into your IDE and the AI agent already knows
the 83 axioms: what to import, what to never import, how to add
input, how to add a sprite, how to deploy. The template stays
readable (≤ 300 lines, ≤ 8000 tokens — F29 & F31) so the whole game
fits in a single context window.

Per axiom F32, the instruction set is versioned in the repo and
mirror-checked in CI. No other Rust gamedev framework does this.

## Three phases of a game

The name comes from the Sanskrit / Pali concept of *trikala* — three
times: past, present, future. The CLI is shaped around them, the
templates' file structure mirrors them, and your error codes carry the
phase prefix (`ATI-001`, `VAR-014`, `ANA-022`).

| Phase | Sanskrit | Action | Command |
|---|---|---|---|
| Born | atita (อดีต) | scaffold | `trikala new` |
| Built | vartamana (ปัจจุบัน) | iterate | `trikala dev` |
| Shipped | anagata (อนาคต) | release | `trikala deploy` |

Pronounced **tri-KAH-lah**.

## What you get in v0.1

- **7 templates**: blank, 2D platformer, 2D top-down, 2D puzzle, board
  game (hex grid, from the 3chess production pattern), card game,
  3D arena. Each ≤ 300 lines of plain Rust.
- **One CLI**: `new`, `dev`, `build`, `deploy`, `claim`, `doctor`,
  `use`, `upgrade`.
- **Anonymous-first deploy**: `trikala deploy` ships a WebAssembly
  build and prints a URL — no account, no signup, no OAuth. The URL
  is yours for 7 days; claim it to keep forever.
- **Hot reload everywhere**: WGSL shaders, sprites, audio, fonts —
  drop a file in `art/`, `music/`, `sfx/` and it's live.
- **Cross-platform**: Windows, macOS, Linux, WebAssembly out of the
  box. Android in v0.2.
- **Production-grade text**: Thai shaping, subpixel RGB, CJK
  fallback — drawn from [3chess.online](https://3chess.online).

## What's deliberately not in it

- **No engine.** No ECS, no scene graph, no plugin system.
- **No abstraction wrappers.** Templates use `wgpu`, `winit`, `kira`,
  `egui` directly — *not* `trikala-render` / `trikala-audio` / etc.
- **No visual editor.** Your IDE is the editor.
- **No mandatory login.** Anonymous deploys work first; auth only
  appears when *you* decide.
- **No telemetry by default.** Opt-in only; nine fields, none of
  which include your code or your project name. See
  [`docs/telemetry-schema.md`](telemetry-schema.md).

## Where it came from

`trikala` is built and maintained by **Round Online**, the same team
behind [**3chess.online**](https://3chess.online) — a real-time
3-player chess game in Rust + wgpu, deployed to desktop, web, and
Android, with a fully encrypted multiplayer protocol (ChaCha20-Poly1305
over a Cloudflare relay, with post-quantum group key agreement via
ML-KEM-768).

The deep technical companion lives at
[**3chess.online/articles**](https://3chess.online) — a living series
on shaders, cross-platform builds, 3-player AI search, Glicko-2 for
3 players, and the parts of online game design that nobody else
writes about in the Rust ecosystem.

## Open source from day one

- **License**: dual MIT / Apache-2.0
- **Contributions**: open from v0.1.0. CONTRIBUTING.md, issue
  templates, good-first-issues — all there at launch.
- **Triage SLA**: 48 hours during the first month.
- **Governance**: axiom-driven. 83 contracts in
  [`trikala-axioms-v1`](trikala-axioms-v1.md) — every change is
  audited against them. Changes require evidence: ≥2x performance,
  ≥10x UX, or existential risk.

## Try it

```sh
curl -fsSL https://trikala.dev/install.sh | sh

trikala new starfighter --template 2d-platformer
cd starfighter
trikala dev          # press F1 for the tuning panel
trikala deploy       # URL in your terminal
```

If that one terminal session puts a sprite on screen and gives you a
URL in under a minute *after the first build*, `trikala` is doing
its job.

---

*Pronounced **tri-KAH-lah**. Made by [Round Online](https://round.online).*
