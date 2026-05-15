# The three phases

trikala's name comes from the Sanskrit / Pali concept of *trikala* —
three times: past, present, future. Every game passes through the
same three. The CLI is shaped around them.

| Phase | Sanskrit | Verb | Command | What you do |
|---|---|---|---|---|
| Born | atita (อดีต) | scaffold | `trikala new` | Choose a template. Get a working, runnable project. |
| Built | vartamana (ปัจจุบัน) | iterate | `trikala dev` | Hot reload. Tune visuals. Add art, sound, levels. |
| Shipped | anagata (อนาคต) | release | `trikala deploy` | Public URL. itch. Steam. Play Store. |

## Why this matters

These aren't decorative chapter titles. They're load-bearing:

- **Help output** (`trikala --help`) groups commands by phase
- **Error codes** carry the phase prefix (`ATI-001`, `VAR-014`, `ANA-022`)
- **The README** of every template is structured around them
- **The contributor model** — a coder, an artist, and a musician
  enter the project in different phases:

| Persona | Mostly works in |
|---|---|
| Coder | Vartamana (`dev`) — gameplay, systems, tuning |
| Visual artist | Atita (project shape) + Vartamana (live shader reload) |
| Musician | Vartamana (`music/` and `sfx/` slots, hot-reloaded) |

Lower the wall between disciplines — that's the whole point.

## Pronunciation

**tri-KAH-lah** — stress on the middle syllable.

Sanskrit roots:
- *tri* — three
- *kala* — time

## Why pick a foreign word

Rust gamedev tooling all sounds like English nouns chosen from a
machine shop (forge, anvil, spark, comfy, bevy). A Sanskrit name is
not ornamental — it carries the project's three-phase architecture
in its bones and gives the brand a moat that any future
"yet-another-game-cli" can't paper over.
