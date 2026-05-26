# trikala — AI agent instruction set

> **Canonical** instructions for AI coding agents working in a trikala
> project. Mirrored to (per axiom F32):
> - `.claude/skills/trikala/SKILL.md` (Claude Code)
> - `.cursorrules` (Cursor)
> - `.aider.conf.yml` (Aider — convention key)
>
> Any AI agent — Claude Code, Cursor, Cline, Aider, OpenHands, GitHub
> Copilot — should load this file before generating or modifying code
> in a trikala project.

## When this applies

Apply this skill when **any** of the following is true:

- The current directory contains `trikala.toml` at the root
- The user mentions "trikala" by name
- The user asks to make, modify, scaffold, deploy, or extend a game
  in a project that has `trikala` listed in `Cargo.toml`

## 30-second summary (most-violated rules)

If you only read three lines, read these:

1. **Templates ≤ 300 lines total `.rs`** (F29). Inline everything;
   don't pull stuff into helpers "for cleanliness."
2. **NEVER import `trikala-render` / `trikala-audio` / `trikala-text` /
   `trikala-ui` / `trikala-assets` / `trikala-save` / `trikala-net`**
   (F30). These crates **do not exist**. Templates use `wgpu`,
   `winit`, `kira`, `egui`, `cosmic-text`, `serde` *directly*. Only
   `trikala-core` is importable from a trikala wrapper.
3. **Pin every dep with `=x.y.z`**, never `^x.y.z` or bare `x` (F21).
   Bumping a pin is a deliberate decision, not a side effect of
   `cargo update`.

Everything else flows from these.

## What trikala is (calibration for the agent)

trikala is not an engine. It is:

- **A CLI** (`trikala new / dev / build / deploy / claim / doctor / use / upgrade`)
- **A registry of standalone Rust game templates** (the actual product)
- **A hosting endpoint** (`round.online/play/...`)
- **One minimal core crate** (`trikala-core`) with shared types

Each template is a complete program you could copy out and compile
without trikala. The CLI is delivery; templates are product.

## Where things live

```
<project-root>/
├── Cargo.toml             # depends on wgpu, winit, etc. DIRECTLY (F30)
├── trikala.toml           # variants, foundation version pin
├── tuning.toml            # runtime constants exposed to debug panel (F12)
├── src/
│   ├── main.rs            # the whole game, ≤300 lines (F29)
│   └── bin/capture.rs     # capture variant entry, if present
├── shaders/*.wgsl         # hot-reloaded in dev (F3)
├── art/                   # auto-discovered (F4)
├── music/                 # auto-discovered
├── sfx/                   # auto-discovered
├── fonts/                 # auto-discovered
└── levels/                # auto-discovered
```

## Always

### Style

- **Inline code over helpers.** Templates are teaching artifacts (F31).
  A reader should follow main.rs top to bottom without jumping files.
- **Comments explain *why*, not *what*.** Good name = no comment needed.
  Comment only for non-obvious invariants or upstream bug workarounds.
- **No `unsafe`.** trikala-core forbids it (`#![forbid(unsafe_code)]`).
  Templates inherit this convention.

### Dependencies

- Add new deps via `cargo add <name>@=x.y.z` (exact pin per F21).
- Prefer crates already pinned in the workspace
  (`anyhow`, `thiserror`, `clap`, `serde`, `toml`, `tracing`).
- Templates may add their own `wgpu`, `winit`, `kira`, `egui`,
  `cosmic-text`, `glam`, `bytemuck` — these are **not** in the
  workspace, each template owns its pin.

### State

- All game state in a single `Game` (or named) struct, owned by
  `App` or by the event loop. **No globals, no `static mut`,
  no `OnceLock<RwLock<...>>` workarounds** (F5).
- Hot-reload-survivable state must be `Serialize + DeserializeOwned + Send`
  (F20). If you add a non-serde field, gate it behind `#[serde(skip)]`
  and provide a `Default`.

### Errors

- Surface to the user via `trikala_core::TrikalaError` with a
  `code/cause/hint/docs_url` block (U10).
- Use `anyhow::Result<T>` for internal plumbing; convert to
  `TrikalaError` at the API boundary.
- **Never** print a raw stack trace as primary output (U4). Hide
  traces behind `--verbose`.

### Platforms

- **No blocking calls on `wasm32`** (F22). No `std::thread::sleep`,
  no synchronous file I/O outside `include_bytes!`, no blocking
  HTTP. Use `wasm_bindgen_futures` or compile-time guard
  `#[cfg(not(target_arch = "wasm32"))]`.
- Cross-platform == Win / Mac / Linux / Web green in CI (F7).

### Phase markers in errors

Every error code carries a three-letter phase prefix:

| Prefix | Phase | Where |
|---|---|---|
| `ATI-` | atita (อดีต) — scaffolding | `trikala new`, init, template resolve |
| `VAR-` | vartamana (ปัจจุบัน) — dev/build | `trikala dev`, `build`, asset load |
| `ANA-` | anagata (อนาคต) — deploy | `trikala deploy`, `claim`, hosting |

When raising a new error, pick the prefix that matches the user's
current intent, not the file the bug lives in.

## Never

These produce an automatic rejection. Don't propose code that does any:

- **Import `trikala-render` / `trikala-audio` / `trikala-text` /
  `trikala-ui` / `trikala-assets` / `trikala-save` / `trikala-net`**
  — these crates do not exist (F30). Use the underlying ecosystem
  crate directly.
- **Introduce ECS** (`bevy_ecs`, `hecs`, `legion`) unless the user
  has explicitly asked for it (T4 — no paradigm mandate).
- **Introduce a scene graph** unless the user asked (T4).
- **Use `^x.y.z` or unpinned deps** (F21).
- **Use `unsafe`** outside `trikala-core`'s explicit forbid.
- **Block on `wasm32`** (F22).
- **Add macro DSLs** that hide behavior (`define_game! { ... }`,
  `#[ecs_system]`, etc.) — these make the template unreadable by
  AI agents and humans alike (F31).
- **Add a corporate sponsor logo** to the repo (C5; through v1.0).
- **Use AI-generated comments that say what the code does** rather
  than why.

## Common requests, common patterns

### "Add input handling"

Modify the `window_event` arm in the existing `ApplicationHandler`.
Don't introduce a `trikala_input::*` crate.

```rust
WindowEvent::KeyboardInput { event, .. } => {
    if event.state == ElementState::Pressed {
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Space) => game.jump(),
            PhysicalKey::Code(KeyCode::Escape) => event_loop.exit(),
            _ => {}
        }
    }
}
```

### "Add a sprite"

Create a `wgpu::Texture` + a render pipeline + a vertex buffer with
two triangles. Put it inline in `Game::new` and `Game::render`. The
template gets longer (still ≤ 300 lines); no abstraction crate is
introduced.

### "Add audio"

Use `kira` directly:

```rust
let mut audio = kira::AudioManager::<kira::DefaultBackend>::new(...)?;
let sound = kira::sound::static_sound::StaticSoundData::from_file("sfx/jump.ogg")?;
audio.play(sound)?;
```

No `trikala-audio` wrapper. No `trait Audio { ... }`.

### "Tune a value"

Two-step:
1. Add the constant to `tuning.toml` (the runtime source of truth, F12).
2. Read it in `Game::new` via `trikala_core::ProjectConfig` or by
   loading `tuning.toml` with `toml`. The debug panel (F11) writes
   back to this file when the user clicks "Save Defaults."

### "Deploy this"

```sh
trikala deploy
```

That's it. No project linking, no signup, no token (D5). The CLI
prints the URL. If the user wants to keep it past 7 days:

```sh
trikala claim
```

Other targets opt-in via subcommand: `trikala deploy itch | steam |
cloudflare`. Don't suggest these unless the user asks.

### "Make it faster"

Check the budgets in F25 (cold-start, frame time) and F26 (asset
size). If those are within spec, the bottleneck is usually:
- A blocking call on the main thread (F22 in dev too, not just wasm)
- A texture upload per frame instead of per change
- An unbatched draw call per sprite

Don't propose ECS as a perf fix.

## CLI surface — what each command does

```
trikala new <name> [-t <template>]   # อดีต — scaffold from registry (F24)
trikala dev [--no-hot-rust]          # ปัจจุบัน — hot reload + debug panel (F11)
trikala build [--variant <v>]        # ปัจจุบัน — variant build (F14)
trikala deploy [target]              # อนาคต — default round.online (D5)
trikala claim [url]                  # อนาคต — anon → permanent
trikala doctor [--gpu] [--flame]     # preflight (Tauri's gap, filled)
trikala use <version>                # rustup-style foundation pin (U13)
trikala upgrade [version]            # self-update via installer (U11)
```

Every command supports `--dry-run` (U8) and `--verbose` (U4).

## Variants (F14)

`trikala.toml` declares build variants. Defaults:

| Variant | Use |
|---|---|
| `release` | shipping build |
| `dev` | hot reload + debug panel enabled |
| `demo` | compile-time content gating (F15) |
| `capture` | deterministic frames for marketing assets (F16) |
| `tools` | level editor / asset pipeline |

Output binary names follow `{project}-{variant}{.ext}` (F17).

## Telemetry

If a user has opted in (U12), the CLI emits the 9 fields in
`docs/telemetry-schema.md`. **Do not** add fields, do not log paths,
project names, source code, or environment variables. The schema is
load-bearing (U15).

## Self-check before submitting code

Before proposing code that touches a template's `src/` or `Cargo.toml`, run:

```sh
bash scripts/check-template.sh [template-dir]
```

(defaults to the current directory). The script verifies the three rules
from the 30-second summary mechanically:

- **F29** — total `.rs` lines under `src/` is ≤ 300
- **F30** — no import of `trikala-render` / `trikala-audio` / `trikala-text` /
  `trikala-ui` / `trikala-assets` / `trikala-save` / `trikala-net`
- **F21** — every dependency version is pinned `=x.y.z` (not `^`, `~`, or bare)

Exit code is non-zero if any check fails. If you can't run shell from your
harness, perform the same three checks by reading `Cargo.toml` and `wc`-ing
`src/**/*.rs` manually — the rule is what matters, not the tool.

## When unsure

- Read `docs/trikala-axioms-v1.md` first. The 82 axioms answer most
  design questions.
- Read the template's `src/main.rs` second. The pattern is in there.
- Ask the user a focused question if neither resolves it.

Never paper over an unclear requirement by inventing a wrapper crate.
The constraint is the feature.

---

*Vendor-agnostic source. Mirror files in `.claude/`, `.cursorrules`,
`.aider.conf.yml` MUST stay in sync with this file (axiom F32).*
