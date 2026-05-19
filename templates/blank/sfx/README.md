# sfx/

Short audio effects. Loaded fully into memory by `kira` so playback
latency is zero.

## Formats

- **OGG Vorbis** — preferred. Short clips compress well.
- **WAV** — uncompressed; OK for very short stings (<200ms).

## Convention

`sfx/jump.ogg` → identifier `"jump"`. Keep clips under ~3 seconds;
longer means it should be in `music/` instead.

## Voicing

Trigger the same `sfx` multiple times concurrently → kira mixes
them. No need to author "variations" unless you want them to sound
distinct (a/b/c randomized picks).
