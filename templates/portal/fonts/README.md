# fonts/

Typefaces for in-game text. `cosmic-text` reads them directly — no
preprocessing, no atlas baking step.

The portal template doesn't render text by default. Add `cosmic-text`
to `Cargo.toml` and drop a TTF/OTF here when you need a HUD, dialog
box, or score display. See the blank template's
[fonts/README.md](../../blank/fonts/README.md) for conventions.
