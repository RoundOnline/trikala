# art/

Drop image files here. Auto-discovered at runtime — no manifest, no
`include_bytes!`, no path constants.

## Formats

- **PNG** — lossless, alpha channel. Best for sprites + UI elements.
- **WebP** — smaller, also lossless. Equivalent for most uses.
- **JPG** — lossy, no alpha. Acceptable for background art only.

## Convention

Filenames become identifiers. `art/player.png` is loaded as `"player"`.
Avoid spaces, use kebab-case for multi-word names (`art/enemy-skull.png`).

## Hot reload

`trikala dev` watches this folder. Save a file → next frame uses the
new version. No restart.

## Size budget

There is no hard limit, but axiom F4 expects auto-discovery to feel
instant. ~50 small sprites is comfortable; if you need thousands of
tiles, consider a single atlas PNG instead.
