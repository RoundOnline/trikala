# `trikala` — Architecture Spec

> สเปกสถาปัตยกรรม. คู่กับ `trikala-axioms-v1.md`.
> Template-as-product model — templates คือ centerpiece, ไม่ใช่ foundation crates

## Vision

**"Templates ที่อ่านได้, fork ได้, deploy ได้ — ภายในคำสั่งเดียว ไม่มี engine ให้เรียน"**

- 2D / 3D / board / card / puzzle / arena — เทมเพลตครอบทุกแนวพื้นฐาน
- ไม่ใช่ engine (ไม่มี ECS / scene graph mandate)
- ไม่มี trikala wrapper crates สำหรับ render/audio/text/ui — templates ใช้ ecosystem ตรง ๆ
- เป็น **"thin core + template registry + opinionated CLI + hosting"**

## Core model

```
trikala = (registry ของ standalone templates)
        + (CLI สำหรับ scaffold / dev / deploy)
        + (round.online สำหรับ ship)
        + (1 minimal core crate สำหรับ shared types)
```

ไม่ใช่ framework. ไม่ใช่ engine. ใกล้กับ "Vercel templates" + "shadcn/ui" + "Bun create" pattern.

## Repo structure (post-pivot)

```
trikala/
├── crates/
│   └── trikala-core/         # minimal: Phase, Error format, Config
├── cli/
│   └── trikala/              # CLI binary
│       └── src/commands/     # new / dev / build / deploy / claim / doctor / use / upgrade
├── templates/                # THE PRODUCT
│   ├── blank/                # ~148 lines, wgpu+winit standalone (compile-verified)
│   ├── 2d-platformer/        # (v0.1.0-alpha.2) ≤300 lines
│   ├── 2d-topdown/           # (v0.1.0-alpha.2)
│   ├── 2d-puzzle/            # (v0.1.0-alpha.2)
│   ├── board-game/           # (v0.1.0-alpha.2) hex grid จาก 3chess pattern
│   ├── card-game/            # (v0.1.0-alpha.2)
│   └── 3d-arena/             # (v0.1.0-alpha.2)
├── examples/                 # snippets ไม่ใช่ template
└── docs/
    ├── trikala-axioms-v1.md
    ├── trikala-architecture.md
    ├── trikala-pitch.md
    ├── phases.md
    ├── quickstart.md
    ├── ai-prompt-cards.md
    ├── hosting-policy.md
    └── telemetry-schema.md
```

= **1 foundation crate + 1 CLI + N templates** (ลดจากเดิมที่มี 8 foundation crates)

## Template anatomy (each template self-contained)

```
templates/<name>/
├── Cargo.toml              # depends on wgpu/winit/kira/egui DIRECTLY (F30)
├── src/main.rs             # ≤300 lines, ≤8000 tokens (F29 + F31)
├── shaders/                # WGSL, hot-reloaded in dev (F3)
├── art/                    # PNG/JPG, hot-reloaded (F4)
├── music/                  # OGG, hot-reloaded
├── sfx/                    # WAV, hot-reloaded
├── trikala.toml            # variants config (F14)
└── .gitignore
```

## Foundation crate (`trikala-core`) — surface

มี **เฉพาะ shared types**:

```rust
pub enum Phase { Atita, Vartamana, Anagata }
pub struct TrikalaError { code, cause, hint, docs_url }   // U10
pub struct ProjectConfig { project, trikala }              // for trikala.toml parsing
pub const TRIKALA_VERSION: &str
```

= ~100–200 บรรทัด total. ไม่มี wgpu wrapper, ไม่มี audio wrapper, ไม่มี ECS

## Ecosystem deps (templates ใช้ตรง ๆ ตาม T8 + F30)

| Need | Template ใช้ตรง | ไม่ผ่าน trikala wrapper |
|---|---|---|
| Window | `winit = "=0.30.5"` | ❌ |
| GPU render | `wgpu = "=27.0.1"` | ❌ |
| Audio | `kira = "=0.10"` (เมื่อ template ต้อง) | ❌ |
| UI | `egui = "=0.33"` | ❌ |
| Math | `glam = "=0.32"` | ❌ |
| Bytes | `bytemuck = "=1.19"` | ❌ |
| Text shaping | `cosmic-text` หรือ `swash` | ❌ |
| Save/load | `serde` + `serde_json` | ❌ |

trikala depend **เฉพาะ**:

| Need | trikala-* ใช้ | ทำไม |
|---|---|---|
| CLI parsing | `clap = "=4.5.23"` | CLI binary only |
| Error format | `thiserror = "=2.0.9"` | core crate |
| Tracing | `tracing = "=0.1.41"` | CLI + core |
| Scaffold engine | `cargo-generate` | shell out จาก CLI |
| Mobile build | `cargo-mobile2` | shell out (v0.2+) |

## CLI surface — final

```
trikala new <name> [-t <template>]   # phase อดีต — scaffold
trikala dev [--no-hot-rust]          # phase ปัจจุบัน — hot reload
trikala build [--variant ...] [--target ...]   # cross-build
trikala deploy [target]              # phase อนาคต — default round.online
trikala claim [url]                  # anonymous → permanent
trikala doctor [--gpu] [--flame]     # preflight + diagnose
trikala use <version>                # version pin (U13)
trikala upgrade [version]            # self-update (U11)
```

Deploy targets:
- (default) `round.online` — anonymous-first (D5), server-mediated.
  User installs only `trikala`; CLI uploads the bundle to our
  endpoint and the server handles hosting. No wrangler, no
  Cloudflare account on the user side.
- `itch` — butler wrapper (alpha.3+)
- `steam` — steamcmd wrapper (v0.2+)
- `static` — `trikala build` already emits a `dist/` folder; users
  who want to host elsewhere can upload it directly. No special
  `trikala deploy static` target needed.
- `android` — Play Console (v0.2+)

## Build variants (F14)

```toml
# trikala.toml ใน user project

[variants.release]
features = []
optimize = "release"
strip    = true

[variants.dev]
features = ["dev-panel", "hot-reload"]
optimize = "debug"

[variants.demo]
features        = ["demo-gate"]
optimize        = "release"
strip           = true
assets_include  = ["assets/demo/**"]
assets_exclude  = ["assets/full/**"]

[variants.capture]
features = ["capture"]
optimize = "release"
entry    = "src/bin/capture.rs"

[variants.tools]
features = ["editor"]
entry    = "src/bin/tools.rs"
```

ทุก variant produce binary ที่ตั้งชื่อชัด: `{name}-{variant}{.ext}` (F17)

## Hosting (round.online) — phased

The hosting backend lives in the private `RoundOnline/trikala-machinery`
repo (server source + ops). The CLI talks to it through a versioned
`/v1/...` HTTP API; users never see Cloudflare directly.

### alpha.2: CLI inner loop, no deploy yet
- `trikala new` / `dev` / `build` work locally
- `trikala deploy` returns a friendly error pointing at this status
- `dist/` from `trikala build` can be uploaded manually to any
  static host as an escape hatch

### alpha.3: round.online server live, anonymous-first
- `trikala deploy` → CLI uploads `dist/` to our endpoint → ephemeral
  URL returned (no signup, no OAuth, no PAT)
- `trikala claim` → GitHub OAuth → URL becomes permanent
- Quota per `hosting-policy.md` (5 games / 100MB / claimed user;
  rate-limit per IP for anonymous deploys)
- Backend: Cloudflare Workers + R2 (bundle storage) + Pages (serving) + D1 (metadata)
- Anonymous URL TTL: 7 days from last deploy

### v0.2: showcase + custom domain
- `round.online/play` gallery of claimed games
- `trikala deploy --domain mygame.com` — DNS CNAME → managed Pages route

## Demo video script (60s — launch hook)

```
[00:00] terminal: trikala new starfighter --template 2d-platformer
[00:05] code editor: src/main.rs opens — 220 lines, you can read it
[00:10] terminal: trikala dev — game window opens, player jumps
[00:15] edit gravity constant in src/main.rs (9.8 → 4.0)
[00:18] hot reload — player floats now
[00:25] edit src/main.rs again — change player color
[00:30] terminal: trikala deploy
[00:50] output: "Live at round.online/play/anon/starfighter ✓ (7-day URL)"
[00:55] click URL — game plays in browser
[01:00] end card:
        "trikala — fork a template, make it yours.
         No engine. No signup. 82 axioms."
```

= viral hook + portfolio signal

## ความสัมพันธ์กับ 3chess production

- **Pattern ที่ port มา** (เป็น template, ไม่ใช่ wrapper): Thai text rendering, hex board logic, shader hot reload pattern, ai_worker channel architecture
- **Pattern ที่อ้างในบทความ แต่ไม่ port**: networking protocol, crypto, MCP, narrative engine
- **ไม่มี code 3chess ดิบใน trikala** — เขียนใหม่ minimal เพื่อ MIT/Apache license (Axiom I1)

## Out of scope (จงใจ — เพื่อกัน scope creep)

- ❌ ECS — user เลือกใช้ `hecs` / `bevy_ecs` เอง
- ❌ Scene graph — function-style แล้วค่อย abstract เอง
- ❌ Editor (visual) — terminal + IDE พอ
- ❌ Physics เต็มรูป — ถ้าต้อง 2D ใช้ `rapier2d` ตามต้องการ
- ❌ AI helpers — มี AI prompt cards ใน docs (C6) แต่ไม่มี SDK บังคับ
- ❌ Multi-player matchmaking — บทความอ้าง 3chess แทน
- ❌ Asset marketplace — community problem ไม่ใช่ tool problem
- ❌ trikala-render / trikala-audio / trikala-text / trikala-ui — ตัดออกตาม F30 (templates ใช้ ecosystem ตรง ๆ)

## Decisions ที่ตัดสินแล้ว

1. ~~Audio crate~~ ✅ templates ใช้ `kira` ตรง ๆ (ไม่มี trikala-audio)
2. ~~Hot reload tech~~ ✅ `notify` crate + `dexterous_developer` (Rust hot reload) optional v0.2
3. ~~Template tooling~~ ✅ `cargo-generate` ใต้ผ้าคลุม (T8)
4. ~~Foundation crate count~~ ✅ 1 (trikala-core) ไม่ใช่ 8
5. `cargo install trikala` build time — ใช้ `cargo-binstall` เป็น primary (target <30s install)

## Versioning

- v0.x = pre-stable, breaking ต่อ minor (F section axioms)
- v1.0 = ตั้งเป้าหลัง 6 เดือนใช้งานจริง + 7 templates ทุกตัว stable
- Semver ตั้งแต่ v1.0
- LTS branch ที่ v1.0 (รับ patch อย่างน้อย 1 ปี)
