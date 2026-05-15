# trikala

> **Copy a game. Make it yours. Ship it in your browser.**
>
> Born, built, shipped — three commands, no signup.
> The first Rust game framework that ships with a Claude Skill.

```sh
curl -fsSL https://trikala.dev/install.sh | sh
# (also: cargo install trikala)

trikala new starfighter --template 2d-platformer
   # → 200 lines of plain Rust in your folder. No engine to learn.

cd starfighter
trikala dev
   # → edit, see it live. Press F1 for the tuning panel.

trikala deploy
   # → https://round.online/play/anon/starfighter
   #   Valid 7 days. `trikala claim` to keep it forever.
```

That's the whole loop.

---

## What it is

A curated registry of **standalone, readable, fork-able game templates**
in Rust + wgpu — plus a CLI that scaffolds, hot-reloads, and ships them.

**Templates are the product.** The CLI is delivery. Hosting closes the loop.

There is **no engine**: no ECS, no scene graph, no plugin system. Each
template is 100–300 lines of plain Rust that uses `wgpu`, `winit`,
`kira`, and `egui` *directly*. You can delete `trikala` tomorrow and
your game still compiles.

This is the [shadcn/ui](https://ui.shadcn.com) pattern, applied to
game development.

## Why this shape

| Audience | Why short, dependency-honest templates win |
|---|---|
| **Beginners** | Read 200 lines, understand the whole game. No abstraction to learn. |
| **AI agents** (Cline, Aider, Cursor, Claude Code) | One context window holds the whole codebase. The agent edits the right line because there's no hidden behavior. |
| **Veterans** | Fork, own, modify, ship. No lock-in. No "trikala 0.4 broke my game." |

## The three phases

Pronounced **tri-KAH-lah** — Sanskrit for *three times*: past, present,
future. The CLI is shaped around them.

| Phase | Sanskrit | Action | Command |
|---|---|---|---|
| Born | atita (อดีต) | scaffold | `trikala new` |
| Built | vartamana (ปัจจุบัน) | iterate | `trikala dev` |
| Shipped | anagata (อนาคต) | release | `trikala deploy` |

## Status

**v0.1.0-alpha.1** — public scaffold. CLI surface is final per axiom
v1; subcommand implementations land in alpha.2.

- [x] Workspace layout, `trikala-core` foundation crate
- [x] CLI surface with all eight verbs (`--help` complete per U5)
- [x] Axiom v1 (83 contracts) — locked
- [x] `AGENT.md` + mirrors for Claude Code / Cursor / Aider
- [x] Architecture spec, pitch, build-variant matrix
- [x] Blank template — standalone 148-line wgpu + winit window
- [ ] 6 more templates (2d-platformer, 2d-topdown, 2d-puzzle, board-game, card-game, 3d-arena)
- [ ] `trikala new` wired against `cargo-generate`
- [ ] `trikala dev` hot-reload loop
- [ ] `trikala build` cross-targets
- [ ] `trikala deploy` to round.online (anonymous-first)
- [ ] `trikala claim`

## Design documents

Read these in order — none of them are long:

1. [`docs/trikala-axioms-v1.md`](docs/trikala-axioms-v1.md) —
   the 82 load-bearing contracts. Every PR is audited against them.
2. [`docs/trikala-architecture.md`](docs/trikala-architecture.md) —
   template catalog, foundation surface, hosting plan.
3. [`docs/trikala-pitch.md`](docs/trikala-pitch.md) —
   the launch artifact.
4. [`docs/phases.md`](docs/phases.md) —
   the three-phase narrative explainer.
5. [`docs/quickstart.md`](docs/quickstart.md) —
   the 60-second tour for new users.

## Three paths into the project

- **You write code**: open any template's `src/main.rs`. Each one is
  self-contained and reads top to bottom. Modify, fork, own.
- **You make visuals**: drop files into `art/` of any template.
  Shaders are WGSL; hot reload is on by default.
- **You make sound**: drop files into `music/` and `sfx/`. The mixer
  is wired by `kira` directly — no trikala-audio wrapper to learn.
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
