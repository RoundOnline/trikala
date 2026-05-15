# Telemetry schema

Concrete data we collect *when a user has explicitly opted in* per
axiom U12. Field-by-field, with the rule from U15 baked in.

Changing this schema (adding, removing, or repurposing a field) is a
**breaking change** and requires a v2 axiom proposal — see axiom U15.

## Default state

**OFF.** Until the user opts in, nothing leaves the machine. The
opt-in banner shows once on first run of `trikala dev` / `trikala
deploy` and never blocks the CLI either way.

Disable forever in any environment with `TRIKALA_TELEMETRY=0`.

## Fields collected (when opted in)

```json
{
  "schema_version": "1",
  "trikala_version": "0.1.0-alpha.1",
  "os_family":      "linux | macos | windows | wasm",
  "arch_family":    "x86_64 | aarch64 | wasm32",
  "session_id":     "<random 128-bit, regenerated per process>",
  "command":        "new | dev | build | deploy | claim | doctor | use | upgrade",
  "exit_code":      0,
  "error_code":     "ATI-001 | VAR-014 | ANA-022 | null",
  "duration_ms":    1234
}
```

That's it. Nine fields.

## Explicitly **not** collected

Per axiom U15:

- Source code, in any form
- File paths or directory names
- Project name
- Asset content (textures, audio, fonts, levels)
- IP address (in long-term storage — used only by CDN for routing, dropped from telemetry pipeline)
- Environment variables
- GitHub username (even after `trikala claim`)
- Command-line flags or arguments

## Aggregation

Long-term store is **aggregated**:
- Counts by `(trikala_version, command, exit_code)`
- Counts by `(os_family, arch_family)`
- Distribution of `duration_ms` per command

`session_id` is **never persisted past 30 days** — it exists to dedupe
within a single dashboard query, not to track users.

## How to inspect what would be sent

```sh
trikala telemetry preview        # prints the JSON the next event would emit
trikala telemetry off            # disable for this machine
trikala telemetry on             # enable for this machine
```

## Why the strict list

Every additional field is a new attack surface, a new privacy
concern, a new excuse for a contributor to leak data without
thinking. The schema is short on purpose.
