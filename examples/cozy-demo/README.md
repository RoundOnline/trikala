# cozy-demo

> A small, cozy 3D action game example — built in the open, in Rust + wgpu.

Walk a low-poly character around an endless, wind-swept dusk meadow:
swap between sword, spear and bow, fight the Withering Warden and the
creatures that roam the grass, and gather the loot they drop — then do
it again, because the enemies always come back. An early prototype on
the way to a cozy strategy-survival sandbox.

Part of the [trikala](../../) repo, built the trikala way: plain
`wgpu` + `winit`, no engine, no ECS (axiom F30).

## Run it

```sh
cargo run -p trikala-cozy-demo
```

The first build compiles `wgpu`, so it takes a few minutes; after that
it starts in seconds.

## Controls

| Input | Action |
|-------|--------|
| `W` `A` `S` `D` | Walk — the character turns to face its heading |
| `Space` | Jump |
| Left mouse / `J` — tap | Quick slash |
| Left mouse / `J` — hold, then release | Charged heavy slash |
| `Esc` | Quit |

Walk onto one of the three weapon pads near the spawn point to equip
that weapon — sword, spear (longer reach) or bow.

## The loop

Step into the meadow and the enemies stir. Their attacks are
telegraphed — a zone with a filling timing bar appears before the blow
lands — so they can be read and dodged. Defeat the Withering Warden or
a monster and it drops gold loot gems; walk over them to collect.
Lose all your health and the character faints, then revives at the
spawn pad. The boss and the monsters respawn after a short delay — the
fight never really ends.

## Under the hood

- A flat-shaded low-poly look — warm key light, hemisphere ambient,
  blob shadows, a graded dusk sky — plus a second flat pipeline for the
  translucent HUD (health bars, loot tally, terrain minimap).
- All geometry is **generated in code**: the meadow, cliffs, trees,
  grass, character, weapons, boss and monsters are built from boxes and
  triangles at runtime. No art assets.
- One file per concern — `main.rs` (game loop), `world.rs` (terrain),
  `grass.rs`, `character.rs`, `weapon.rs`, `boss.rs`, `monster.rs`,
  `hud.rs`, `geometry.rs` — plus the shaders `scene.wgsl` and
  `hud.wgsl`.

## Status

Early prototype, built in public and iterating fast. Expect rough
edges — and expect it to change.

## License

Dual-licensed under MIT or Apache-2.0, the same as the rest of the
trikala repo — see [`LICENSE-MIT`](../../LICENSE-MIT) and
[`LICENSE-APACHE`](../../LICENSE-APACHE) at the repo root.
