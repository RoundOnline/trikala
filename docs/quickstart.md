# Quickstart

The 60-second tour. If anything here takes longer than promised,
file an issue — that's a regression against axiom U2 / U3.

## Install

```sh
# Recommended (axiom U11 — installer is the primary path)
curl -fsSL https://trikala.dev/install.sh | sh

# Also works (slower; cargo build from source)
cargo install trikala
```

## The whole loop

```sh
trikala new starfighter           # อดีต — scaffold
cd starfighter

trikala dev                       # ปัจจุบัน — local game, hot reload
# (edit src/main.rs or any file in shaders/, art/, music/ — instant update)

trikala deploy                    # อนาคต — ephemeral URL on round.online
# >> https://round.online/play/anon/starfighter (valid 7 days)
```

That's it. No signup, no project linking, no `git push` before you
can show someone.

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

## Tune visuals without recompiling

Press **F1** in `trikala dev` to open the debug panel. Adjust sliders.
Click **Save Defaults** — values are written back to `tuning.toml`,
which you commit to git. The next `trikala dev` boots with your tuned
values.

## Ship to other targets

```sh
trikala deploy itch              # butler under the hood
trikala deploy steam             # steamcmd under the hood
trikala deploy cloudflare        # wrangler, bring your own CF_API_TOKEN
trikala build --variant demo     # produce a demo binary
trikala build --variant capture  # produce a capture binary
```

## Troubleshooting

```sh
trikala doctor                   # preflight: rust, cargo, wgpu, network
trikala doctor --gpu             # extra GPU benchmark
trikala doctor --flame           # flamegraph of the current build
```

Every error trikala emits has a code like `ATI-001`, a one-line
cause, a one-line hint, and a `docs.trikala.dev` URL. If you see
a stack trace by default, that's a regression against axiom U4.
