# Asset credits

## character.glb — Erika Archer + Mixamo Locomotion Pack

Source: https://www.mixamo.com/ (Adobe — free with account)

Character: Erika Archer (from Mixamo's character library)
Animations: Mixamo "Locomotion Pack" (with skin on the base
character, animation-only for the rest), retargeted to the same
skeleton because every Mixamo clip ships on the same `mixamorig:*`
rig.

License: Free for use in custom projects per Mixamo Terms of Use.
Adobe permits commercial and non-commercial use of Mixamo content;
redistribution as a standalone asset is restricted, so this file
is bundled in the project for execution only.

Bundled clips (rebound to state-machine-friendly names):
- Idle
- Walk
- Run
- Jump
- StrafeLeft / StrafeRight
- StrafeLeftWalk / StrafeRightWalk
- TurnLeft / TurnRight
- TurnLeft90 / TurnRight90

The state machine in `src/main.rs` consumes these clip names via
`resolve_state_clip()`. To swap in a different character, name the
exported clips the same way (or extend the lookup table).

See `tools/build_character.md` for the merge pipeline used to
produce this single GLB from many Mixamo FBX downloads.
