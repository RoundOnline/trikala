# music/

Background tracks. Long-form audio, streamed by `kira` from disk so
RAM stays clean.

## Formats

- **OGG Vorbis** — preferred. Cross-platform, good compression.
- **FLAC** — lossless, larger. Use when fidelity matters.
- **MP3** — works but encumbered patent history; prefer OGG.

## Convention

`music/main-theme.ogg` is loaded as `"main-theme"`. Pass that name
to your audio mixer call. Hot reload in `trikala dev`.

## Looping

Most loops want a seamless boundary. Author the file to start and
end on the same beat. trikala doesn't crossfade for you; that's a
deliberate choice you make in code (see `kira::TrackBuilder`).
