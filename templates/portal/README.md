# {{project-name}}

A trikala portal game. Born from `templates/portal`, owned by you.

Two rooms, a door between them, a cube you control with WASD.
Step into the door and the world flips. The door itself is a
window into the *other* room — what you see in the doorframe is
literally the destination, rendered with a virtual camera matching
yours.

## Run it

```sh
trikala dev       # iterate with hot reload
cargo run         # plain cargo also works — trikala doesn't lock you in
```

WASD to move. Mouse for camera. Walk into the door at the far end.

## What the portal does

```
┌─────────────┐ ←  door's "open" side       ┌─────────────┐
│   room 0    │                              │   room 1    │
│   (blue)    │  ← step through this side ─→ │   (green)   │
│             │                              │             │
└──────┬──────┘                              └──────┬──────┘
       │                                            │
       └────────────── one Doraemon door ───────────┘
                       (same world-space position,
                        renders as a window into the
                        room you're NOT in)
```

The render-to-texture trick is in `src/main.rs:render()`. Two
passes per frame: first render the destination room into an
offscreen colour target; then render the current room normally,
with a quad inside the door that samples from that colour target.
Crossing the door's z-plane flips `current_world` and the roles
swap on the next frame.

## Project layout

```
{{project-name}}/
├── Cargo.toml     # depends on wgpu/winit/glam/bytemuck DIRECTLY (no trikala-*)
├── trikala.toml   # variants + foundation pin (only trikala CLI reads this)
├── src/main.rs    # the whole game — ≤700 lines, one file
├── art/           # PNG/WebP — auto-discovered
├── music/         # OGG — auto-discovered
├── sfx/           # OGG/WAV — auto-discovered
└── fonts/         # TTF/OTF — first one is default
```

You can delete `trikala` tomorrow and `cargo run` will still work.
That is the design (axiom T1 + F30).

## Ship it

```sh
trikala build     # release binary + dist/
trikala deploy    # public URL — anonymous, 7-day default (alpha.3+)
```

## License

Yours to choose. The trikala template was dual-licensed MIT / Apache-2.0;
your fork inherits those licenses unless you change them.
