# Quickstart

The 60-second tour. If anything here takes longer than promised,
file an issue — that's a regression against axiom U2 / U3.

## Install

```sh
# macOS / Linux
curl -fsSL https://trikala.round.online/install.sh | sh

# Windows (PowerShell)
irm https://trikala.round.online/install.ps1 | iex
```

The installer drops a single binary into `~/.local/bin` (or
`%LOCALAPPDATA%\trikala\bin` on Windows). No Rust toolchain needed
for the install itself — but you'll need one
([rustup.rs](https://rustup.rs)) to run `trikala dev` since the
scaffolded project is a normal Cargo project.

## The whole loop

```sh
trikala new starfighter           # อดีต — scaffold
cd starfighter

trikala dev                       # ปัจจุบัน — runs cargo run + watches src/
# (edit src/main.rs and save — the watcher kills the binary and re-runs)

trikala build                     # อนาคต — release binary + assets in dist/
trikala deploy                    # ephemeral URL (alpha.3+)
```

`trikala deploy` is server-mediated: the CLI uploads `dist/` to our
endpoint and we host it. No wrangler, no Cloudflare account on your
side. Lands in alpha.3.

## Keep the URL

When you decide your game is worth keeping:

```sh
trikala claim
# opens browser → GitHub OAuth → URL is now permanent
```

## Pick a different starter

```sh
trikala new mygame --template 2d-platformer
# or: --template board-game | 2d-topdown | 2d-puzzle | card-game | 3d-arena
```

(In alpha.2 only `blank` is shipped. The list above grows in alpha.3.)

## What's in your project

```
my-game/
├── Cargo.toml              # depends on wgpu / winit / kira / egui DIRECTLY (axiom F30)
├── trikala.toml            # variants + foundation pin
├── src/main.rs             # the whole game, ≤300 lines (axiom F29)
├── art/                    # PNG / WebP — auto-discovered (axiom F4)
├── music/                  # OGG — auto-discovered
├── sfx/                    # OGG / WAV — auto-discovered
└── fonts/                  # TTF / OTF — first one becomes default
```

Each subfolder has a `README.md` documenting format conventions and
hot-reload behavior.

## Tune visuals without recompiling

(Lands in alpha.3 alongside hot-reload work) Press **F1** in
`trikala dev` to open the debug panel. Adjust sliders. Click
**Save Defaults** — values are written back to `tuning.toml`, which
you commit to git. The next `trikala dev` boots with your tuned
values.

## Troubleshooting

```sh
trikala doctor                   # preflight: rust, cargo, wgpu, network
trikala doctor --gpu             # extra GPU benchmark
trikala doctor --flame           # flamegraph of the current build
```

(Doctor's full checks land in alpha.3 — for now it prints the list
of things it *will* check.)

Every error trikala emits has a code like `ATI-001`, a one-line
cause, a one-line hint, and a `trikala.round.online/errors/...`
URL. If you see a stack trace by default, that's a regression
against axiom U4.
