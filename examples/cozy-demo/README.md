# cozy-demo

> A small, cozy 3D game example — built in the open, in Rust + wgpu.

Walk a little low-poly character around a stepped meadow at dusk: jump
up onto hills and trees, and swing a sword with two kinds of attack.
An early prototype on the way to a cozy strategy-survival sandbox.

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

## Under the hood

- One render pipeline, one flat-shaded low-poly look: a warm key light
  plus hemisphere ambient, blob shadows, and a graded dusk sky.
- All geometry is **generated in code** — the meadow, the cliffs, the
  trees, the character and its sword are built from boxes at runtime.
  No art assets yet; those come later.
- The whole game is two files: [`src/main.rs`](src/main.rs) and the
  shader [`src/scene.wgsl`](src/scene.wgsl).

## Status

Early prototype, built in public and iterating fast. Expect rough
edges — and expect it to change.

## License

Dual-licensed under MIT or Apache-2.0, the same as the rest of the
trikala repo — see [`LICENSE-MIT`](../../LICENSE-MIT) and
[`LICENSE-APACHE`](../../LICENSE-APACHE) at the repo root.
