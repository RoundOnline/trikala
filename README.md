# trikala

> **Fork a template. Make it yours. Ship it.**
>
> Born, built, shipped — three commands, no signup.
> The first Rust game CLI that ships with an AI-agent skill.

## Install

trikala is in alpha — clone the repo and build it from source:

```sh
git clone https://github.com/RoundOnline/trikala
cd trikala
cargo run -p trikala-hello
```

It's a standard Cargo workspace — you need Rust 1.78+
([rustup.rs](https://rustup.rs)) and ~2 GB of free disk for the build
cache (the first build takes ~5 min). Works on macOS, Linux and Windows.

## Try it

```sh
trikala new starfighter
cd starfighter
trikala dev
```

A window opens, cycling through colors. That's the `blank` template
— ~150 lines of plain Rust + wgpu + winit. Every other template is a
fork of this one. Edit `src/main.rs`, save, and the watcher restarts
the binary.

When ready:

```sh
trikala build       # release binary + assets in dist/
trikala deploy      # public URL, anonymous, 7-day default
                    # `trikala claim` to keep forever
```

## What's in alpha.2

- ✅ `trikala new <name>` — scaffolds from the embedded `blank`
  template. No network call at scaffold time.
- ✅ `trikala dev` — wraps `cargo run`, watches `src/` + `shaders/`,
  restarts on save. Press Ctrl+C to stop.
- ✅ `trikala build` — wraps `cargo build --release`, copies binary
  plus `art/ music/ sfx/ fonts/` into `dist/`.
- ✅ `templates/blank` — wgpu + winit window + asset-folder structure
  with READMEs explaining where to drop files.
- ✅ `examples/hello` — runnable preview of the blank template
  (`cargo run -p trikala-hello` clones not required).
- ✅ `examples/mood` — multi-room portal prototype with skinned
  character, dream-blend transitions, fireflies, footprint decals,
  4-room landscape. Read it as a reference for what trikala-scale
  projects look like.
- ✅ `AGENT.md` — AI agents (Claude, Cursor, Cline, Aider) load this
  to know trikala's 83 axioms before generating code.
- ⏳ `trikala deploy` + `trikala claim` — server-mediated hosting
  (no wrangler, no Cloudflare account on your side). Lands when the
  trikala-server backend is ready.
- ⏳ `examples/showroom` — full landscape + 3D + font + HUD demo.
  See [`docs/showroom-plan.md`](docs/showroom-plan.md).
- ⏳ 6 more templates: 2d-platformer, 2d-topdown, 2d-puzzle,
  board-game, card-game, 3d-arena. Each ≤300 lines (axiom F29).

## Which game are you making?

You're a developer, but you're also a gamer — pick the direction
that matches the game in your head:

| You want to focus on... | Template | Status |
|---|---|---|
| **Gameplay / mechanics** | `2d-platformer`, `2d-topdown` | alpha.2+ |
| **Story / narrative** | `visual-novel`, `card-game` | alpha.3 |
| **Visual fidelity / 3D** | `3d-arena` | alpha.2+ |
| **Board / strategy** | `board-game` (hex grid, from 3chess pattern) | alpha.2+ |
| **Not sure yet** | `examples/showroom` — see everything in one window | stub today |

The showroom is the entry point for the undecided: one window will
show landscape + 3D model + font + HUD UI, so you see the language
of the framework before committing to a direction.

## What it is

A curated registry of **standalone, readable, fork-able game
templates** in Rust + wgpu — plus a CLI that scaffolds, hot-reloads,
and ships them.

**Templates are the product.** The CLI is delivery. Hosting closes
the loop.

There is **no engine**: no ECS, no scene graph, no plugin system.
Each template is 100–300 lines of plain Rust that uses `wgpu`,
`winit`, `kira`, and `egui` *directly* (axiom F30 — no `trikala-render`
/ `trikala-audio` / etc.). You can delete `trikala` tomorrow and
`cargo run` still works.

## Why this shape

| Audience | Why short, dependency-honest templates win |
|---|---|
| **Beginners** | Read 200 lines, understand the whole game. No abstraction to learn. |
| **AI agents** (Claude Code, Cursor, Cline, Aider) | One context window holds the whole codebase. The agent edits the right line because there's no hidden behavior. |
| **Veterans** | Fork, own, modify, ship. No lock-in. No "trikala 0.4 broke my game." |

## The three phases

Pronounced **tri-KAH-lah** — Sanskrit for *three times*: past, present,
future. The CLI is shaped around them.

| Phase | Sanskrit | Action | Command |
|---|---|---|---|
| Born | atita (อดีต) | scaffold | `trikala new` |
| Built | vartamana (ปัจจุบัน) | iterate | `trikala dev` |
| Shipped | anagata (อนาคต) | release | `trikala build` + `trikala deploy` |

## Design documents

Read these in order — none of them are long:

1. [`docs/trikala-axioms-v1.md`](docs/trikala-axioms-v1.md) —
   the 83 load-bearing contracts. Every PR is audited against them.
2. [`docs/trikala-architecture.md`](docs/trikala-architecture.md) —
   template catalog, foundation surface, hosting plan.
3. [`docs/trikala-pitch.md`](docs/trikala-pitch.md) —
   the launch artifact.
4. [`docs/phases.md`](docs/phases.md) —
   the three-phase narrative explainer.
5. [`docs/quickstart.md`](docs/quickstart.md) —
   the 60-second tour for new users.

## Three paths into the project

- **You write code**: open any template's `src/main.rs`. Each one
  reads top to bottom in a single file. Modify, fork, own.
- **You make visuals**: drop files into `art/` of the scaffolded
  project. Shaders are WGSL; hot reload is on by default.
- **You make sound**: drop files into `music/` and `sfx/`. The mixer
  is wired by `kira` directly — no `trikala-audio` wrapper to learn.
- **You pair-program with an AI**: open the project in Claude Code,
  Cursor, Cline, or Aider — the agent already knows trikala's 83
  axioms via `AGENT.md` and the vendor-specific mirror files.

## License

Dual-licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT))
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

at your option.

## Contributing

Open from day one — see [CONTRIBUTING.md](CONTRIBUTING.md).
First-time contributors: check the `good-first-issue` label.

---

*Made by [Round Online](https://round.online).
[3chess.online](https://3chess.online) — our reference production
game, written in the same stack and the source of every pattern
ported into trikala.*
