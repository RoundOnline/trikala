use crate::character::Character;

// Animation state machine — selects which named clip plays and
// cross-fades between consecutive states over a short transition.
// Clip names are the ones baked into the bundled character.glb by
// `tools/build_character.md` (Mixamo Erika Archer + locomotion pack).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AnimState {
    Idle,
    Walk,
    Run,
    Jump,
}

pub fn resolve_state_clip(state: AnimState, character: &Character) -> Option<&str> {
    match state {
        AnimState::Idle => character.find_clip(&["Idle", "Standing", "Stand"]),
        AnimState::Walk => character.walk_clip_name(),
        AnimState::Run  => character.find_clip(&["Run", "Running", "Jog", "Jogging"]),
        AnimState::Jump => character.find_clip(&["Jump", "Jumping"]),
    }
}

pub struct AnimController {
    pub state: AnimState,
    pub state_time: f32,
    pub previous: Option<AnimState>,
    pub previous_time: f32,
    /// 0..1 — 1 = transition complete.
    pub transition: f32,
    pub transition_duration: f32,
}

impl AnimController {
    pub fn new() -> Self {
        Self {
            state: AnimState::Idle,
            state_time: 0.0,
            previous: None,
            previous_time: 0.0,
            transition: 1.0,
            transition_duration: 0.18,
        }
    }

    pub fn set(&mut self, new_state: AnimState) {
        if new_state == self.state {
            return;
        }
        println!("[anim] {:?} -> {:?}", self.state, new_state);
        self.previous = Some(self.state);
        self.previous_time = self.state_time;
        self.state = new_state;
        self.state_time = 0.0;
        self.transition = 0.0;
    }

    pub fn tick(&mut self, dt: f32, state_rate: f32) {
        self.state_time += state_rate * dt;
        if self.transition < 1.0 {
            self.transition = (self.transition + dt / self.transition_duration).min(1.0);
            if self.transition >= 1.0 {
                self.previous = None;
            }
        }
    }
}
