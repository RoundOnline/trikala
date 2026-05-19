# {{project-name}}

A trikala game project. Born from `templates/blank`, owned by you.

## Run it

```sh
trikala dev       # iterate with hot reload
cargo run         # plain cargo also works — trikala doesn't lock you in
```

## Project layout

```
{{project-name}}/
├── Cargo.toml     # depends on wgpu/winit/kira/egui DIRECTLY (no trikala-*)
├── trikala.toml   # variants, foundation pin (only trikala CLI reads this)
├── src/main.rs    # the whole game — read top to bottom, ≤300 lines
├── art/           # sprites, textures (.png, .webp) — drop files, auto-discovered
├── music/         # background tracks (.ogg preferred)
├── sfx/           # short effects (.ogg / .wav)
└── fonts/         # text rendering (.ttf / .otf, first one becomes default)
```

You can delete `trikala` tomorrow and `cargo run` will still work.
That is the design (axiom T1 + F30).

## Ship it

```sh
trikala build     # release binary in target/release/
trikala deploy    # public URL — anonymous, 7-day default, claim to keep
```

## License

Yours to choose. The trikala template was dual-licensed MIT / Apache-2.0;
your fork inherits those licenses unless you change them.
