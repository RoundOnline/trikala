# Asset credits

## character.glb — RobotExpressive

Source: https://github.com/mrdoob/three.js/tree/dev/examples/models/gltf/RobotExpressive

Author: Tomás Laulhé (https://www.patreon.com/quaternius)
Modifications: donmccurdy
License: CC0 1.0 Universal (https://creativecommons.org/publicdomain/zero/1.0/)

Animations bundled in the asset:
- Idle, Walking, Running
- Jump, WalkJump
- Punch, ThumbsUp, Wave, Yes, No
- Dance, Death, Sitting, Standing

The character state machine in `src/main.rs` maps gameplay states
to clip names (e.g. AnimState::Attack → "Punch"). To use a Mixamo
download instead, name your exported clips matching the lookup
table in `resolve_state_clip()`.
