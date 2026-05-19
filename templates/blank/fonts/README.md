# fonts/

Typefaces for in-game text. `cosmic-text` reads them directly — no
preprocessing, no atlas baking step.

## Formats

- **TTF** — works.
- **OTF** — works.
- **WOFF2** — also works, smaller files.

## Default

The **first** font discovered alphabetically becomes the default,
unless your code explicitly names another. Drop `default.ttf` to
make it explicit.

## CJK / Thai / emoji

`cosmic-text` can chain multiple fonts as fallbacks for missing
glyphs. Drop additional font files for the scripts you need and
configure the fallback order in code.

## Licensing

Most free fonts are under the **SIL Open Font License (OFL)**. You
must ship the OFL text with your binary if you redistribute. Check
each font's license file.
