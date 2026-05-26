# Quickstart

The 60-second tour. If anything here takes longer than promised,
file an issue — that's a regression against axiom U2 / U3.

## Install

trikala is in alpha — clone the repo and build it from source:

```sh
git clone https://github.com/RoundOnline/trikala
cd trikala
cargo run -p trikala-hello
```

Standard Cargo workspace — you need Rust 1.78+
([rustup.rs](https://rustup.rs)). First build takes ~5 minutes;
subsequent builds are seconds. Works on macOS, Linux, and Windows.

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
cause, a one-line hint, and a link to a GitHub issues search for
that code (so you can see if anyone else has hit it). If you see a
stack trace by default, that's a regression against axiom U4.
