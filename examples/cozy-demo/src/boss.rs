//! The boss — the Withering Warden, a giant ancient treant. It rests
//! as the largest tree on the meadow until the player draws near, then
//! wakes, chases, and slams. It flashes when struck and, once its
//! light is spent, calms and settles for good.

use crate::character::sample;
use crate::geometry::{flash_color, push_box, push_box_emissive, push_rotated, Vertex, FLAT_NORMAL};
use crate::world::{solid_height, tile_height};
use glam::{Mat4, Vec3};

const AGGRO_RANGE: f32 = 11.0;
const DEAGGRO_RANGE: f32 = 22.0;
const ATTACK_RANGE: f32 = 5.0;
const AIM_TOLERANCE: f32 = 0.18; // boss must face within this angle (rad) before it commits
const MOVE_SPEED: f32 = 3.6;
const TURN_SPEED: f32 = 3.2;
const ATTACK_DUR: f32 = 1.3;
const RECOVER_DUR: f32 = 0.8;
const STRIKE_FRAC: f32 = 0.62; // point in the slam where the blow lands
const ZONE_REACH: f32 = 3.4; // how far ahead of the boss the slam lands
const DAMAGE_RADIUS: f32 = 2.7;
const HIT_FLASH_DUR: f32 = 0.22;
const MAX_HP: f32 = 12.0;

/// Footprint radius — used by the boss's own collision and the player's.
pub const BOSS_RADIUS: f32 = 2.0;

/// Slam keyframes (time 0..1, attacking-arm x-angle): a long windup, a
/// fast strike, then an impact hold — the player's weight curve, scaled.
const SLAM_X: [(f32, f32); 5] = [
    (0.00, 0.2),
    (0.45, 2.7),
    (0.62, -1.3),
    (0.76, -1.35),
    (1.00, 0.2),
];

#[derive(Copy, Clone, PartialEq)]
enum State {
    Sleep,
    Chase,
    Attack,
    Recover,
    Calm,
}

pub struct Boss {
    pos: Vec3,
    facing: f32,
    state: State,
    timer: f32,  // time within the current Attack / Recover
    anim_t: f32, // ever-advancing, drives idle sway + the glow pulse
    hp: f32,
    hit_flash: f32,
    struck: bool, // true the one frame the slam connects
    /// Where the slam will land — locked when the attack commits, so
    /// the telegraph stays put and the player can step clear.
    attack_center: Vec3,
}

impl Boss {
    pub fn new(x: f32, z: f32) -> Self {
        Self {
            pos: Vec3::new(x, tile_height(x.floor() as i32, z.floor() as i32), z),
            facing: 0.0,
            state: State::Sleep,
            timer: 0.0,
            anim_t: 0.0,
            hp: MAX_HP,
            hit_flash: 0.0,
            struck: false,
            attack_center: Vec3::ZERO,
        }
    }

    pub fn pos(&self) -> Vec3 {
        self.pos
    }

    /// True for the single frame the slam lands — test the player
    /// against `attack_zone` when this is set.
    pub fn struck(&self) -> bool {
        self.struck
    }

    /// The slam's damage circle (centre, radius), while attacking.
    pub fn attack_zone(&self) -> Option<(Vec3, f32)> {
        if self.state == State::Attack {
            Some((self.attack_center, DAMAGE_RADIUS))
        } else {
            None
        }
    }

    /// Take a sword hit — flash white, recoil, and once spent, calm.
    pub fn take_hit(&mut self, dmg: f32) {
        if self.state == State::Calm {
            return;
        }
        self.hp -= dmg;
        self.hit_flash = HIT_FLASH_DUR;
    }

    /// Advance the boss: wake near the player, chase, slam, recover.
    pub fn update(&mut self, dt: f32, player: Vec3) {
        self.struck = false;
        self.anim_t += dt;
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        // stay grounded on the procedural terrain
        self.pos.y = tile_height(self.pos.x.floor() as i32, self.pos.z.floor() as i32);

        if self.hp <= 0.0 {
            self.state = State::Calm;
        }
        if self.state == State::Calm {
            return;
        }

        let to_player = Vec3::new(player.x - self.pos.x, 0.0, player.z - self.pos.z);
        let dist = to_player.length();

        match self.state {
            State::Sleep => {
                if dist < AGGRO_RANGE {
                    self.state = State::Chase;
                }
            }
            State::Chase => {
                self.turn_toward(to_player, TURN_SPEED * dt);
                // how far the boss still has to turn to face the player
                let mut d = to_player.x.atan2(to_player.z) - self.facing;
                let pi = std::f32::consts::PI;
                while d > pi { d -= std::f32::consts::TAU; }
                while d < -pi { d += std::f32::consts::TAU; }
                let aim = d.abs();
                if dist > DEAGGRO_RANGE {
                    self.state = State::Sleep;
                } else if dist < ATTACK_RANGE && aim < AIM_TOLERANCE {
                    // in range and lined up — commit, and lock the slam
                    // spot so it no longer tracks the player
                    self.state = State::Attack;
                    self.timer = 0.0;
                    let fwd = Vec3::new(self.facing.sin(), 0.0, self.facing.cos());
                    self.attack_center = self.pos + fwd * ZONE_REACH;
                } else if dist >= ATTACK_RANGE && dist > 0.01 {
                    // not in range yet — close the distance (turning above)
                    self.step(to_player / dist * MOVE_SPEED * dt);
                }
                // in range but not yet facing the player: keep turning
            }
            State::Attack => {
                let strike = STRIKE_FRAC * ATTACK_DUR;
                let prev = self.timer;
                self.timer += dt;
                if prev < strike && self.timer >= strike {
                    self.struck = true;
                }
                if self.timer >= ATTACK_DUR {
                    self.state = State::Recover;
                    self.timer = 0.0;
                }
            }
            State::Recover => {
                self.timer += dt;
                if self.timer >= RECOVER_DUR {
                    self.state = if dist < AGGRO_RANGE { State::Chase } else { State::Sleep };
                }
            }
            State::Calm => {}
        }
    }

    /// Move by `step`, but not through cliffs or trees (axis-separated).
    fn step(&mut self, step: Vec3) {
        if step.x.abs() > 1e-5 {
            let probe = self.pos.x + step.x + step.x.signum() * BOSS_RADIUS;
            if solid_height(probe, self.pos.z) <= self.pos.y + 1.5 {
                self.pos.x += step.x;
            }
        }
        if step.z.abs() > 1e-5 {
            let probe = self.pos.z + step.z + step.z.signum() * BOSS_RADIUS;
            if solid_height(self.pos.x, probe) <= self.pos.y + 1.5 {
                self.pos.z += step.z;
            }
        }
    }

    fn turn_toward(&mut self, to_player: Vec3, max_step: f32) {
        if to_player.length_squared() < 1e-4 {
            return;
        }
        let target = to_player.x.atan2(to_player.z);
        let mut d = target - self.facing;
        let pi = std::f32::consts::PI;
        while d > pi { d -= std::f32::consts::TAU; }
        while d < -pi { d += std::f32::consts::TAU; }
        self.facing += d.clamp(-max_step, max_step);
    }

    /// Append the posed boss — and its attack telegraph — to `out`.
    pub fn mesh(&self, out: &mut Vec<Vertex>) {
        // damage telegraph — a ground disc that brightens through the
        // windup, then flashes white the instant the slam lands
        if self.state == State::Attack {
            let c = self.attack_center + Vec3::new(0.0, 0.06, 0.0);
            let wp = (self.timer / ATTACK_DUR).clamp(0.0, 1.0);
            let col = if wp >= STRIKE_FRAC && wp < STRIKE_FRAC + 0.14 {
                [1.0, 0.95, 0.7]
            } else {
                let k = (wp / STRIKE_FRAC).min(1.0);
                [0.5 + 0.5 * k, 0.16 + 0.10 * k, 0.06]
            };
            let seg = 16;
            for i in 0..seg {
                let a0 = i as f32 / seg as f32 * std::f32::consts::TAU;
                let a1 = (i + 1) as f32 / seg as f32 * std::f32::consts::TAU;
                let p0 = c + Vec3::new(a0.cos() * DAMAGE_RADIUS, 0.0, a0.sin() * DAMAGE_RADIUS);
                let p1 = c + Vec3::new(a1.cos() * DAMAGE_RADIUS, 0.0, a1.sin() * DAMAGE_RADIUS);
                for p in [c, p0, p1] {
                    out.push(Vertex { pos: p.to_array(), normal: FLAT_NORMAL, color: col });
                }
            }
        }

        let bark = [0.22, 0.16, 0.13];
        let bark_dark = [0.17, 0.12, 0.10];
        let leaf = [0.24, 0.40, 0.25];
        let dead = [0.42, 0.41, 0.36];
        let face_dark = [0.09, 0.07, 0.06];
        // a slow pulse of the inner light — dimmer once the boss is calmed
        let alive = if self.state == State::Calm { 0.4 } else { 1.0 };
        let p = (0.7 + 0.3 * (self.anim_t * 1.3).sin()) * alive;
        let glow = [1.0 * p, 0.72 * p, 0.34 * p];

        let mut local: Vec<Vertex> = Vec::new();

        // roots — flat slabs spreading from the base
        push_box(&mut local, Vec3::new(0.0, 0.22, 0.0), Vec3::new(1.5, 0.22, 1.3), bark_dark);
        for (rx, rz) in [(-1.5, 0.3), (1.5, 0.3), (0.3, -1.4), (0.3, 1.4)] {
            push_box(&mut local, Vec3::new(rx, 0.13, rz), Vec3::new(0.7, 0.13, 0.6), bark_dark);
        }

        // trunk — three tapering boxes
        push_box(&mut local, Vec3::new(0.0, 1.30, 0.0), Vec3::new(1.15, 1.05, 0.95), bark);
        push_box(&mut local, Vec3::new(0.0, 3.00, 0.0), Vec3::new(0.90, 0.85, 0.78), bark);
        push_box(&mut local, Vec3::new(0.0, 4.20, 0.0), Vec3::new(0.72, 0.62, 0.62), bark);

        // a face carved into the upper trunk (+Z side), with glowing eyes
        push_box(&mut local, Vec3::new(0.0, 4.35, 0.62), Vec3::new(0.62, 0.10, 0.10), face_dark);
        push_box(&mut local, Vec3::new(0.0, 3.70, 0.64), Vec3::new(0.34, 0.08, 0.07), face_dark);
        for ex in [-0.30, 0.30] {
            push_box_emissive(&mut local, Vec3::new(ex, 4.05, 0.66), Vec3::new(0.14, 0.12, 0.09), glow);
        }

        // glowing cracks — the dying light leaking from the bark
        push_box_emissive(&mut local, Vec3::new(0.35, 2.0, 0.95), Vec3::new(0.07, 0.50, 0.04), glow);
        push_box_emissive(&mut local, Vec3::new(-0.45, 3.2, 0.78), Vec3::new(0.06, 0.42, 0.04), glow);
        push_box_emissive(&mut local, Vec3::new(0.15, 1.0, 1.00), Vec3::new(0.06, 0.35, 0.04), glow);

        // crown — asymmetric: lush green on one side, dead grey on the other
        for (cx, cy, cz, s) in [(-0.80, 5.30, 0.00, 0.95), (-0.40, 6.40, 0.15, 0.72), (-0.85, 5.80, -0.50, 0.60)] {
            push_box(&mut local, Vec3::new(cx, cy, cz), Vec3::splat(s), leaf);
        }
        for (cx, cy, cz, s) in [(0.85, 5.20, 0.00, 0.78), (0.60, 6.10, -0.25, 0.52), (1.00, 5.60, 0.40, 0.46)] {
            push_box(&mut local, Vec3::new(cx, cy, cz), Vec3::splat(s), dead);
        }

        // arms — the right one slams, the left one drifts gently
        let drift = (self.anim_t * 0.5).sin() * 0.05;
        push_warden_arm(&mut local, -1.45, Mat4::from_rotation_x(drift), bark, bark_dark);
        let right = match self.state {
            State::Attack => Mat4::from_rotation_x(sample(&SLAM_X, self.timer / ATTACK_DUR)),
            _ => Mat4::from_rotation_x(0.2 + drift),
        };
        push_warden_arm(&mut local, 1.45, right, bark, bark_dark);

        // place it: gentle sway, a forward lean on the slam, a recoil
        // back when struck, then face the player and move into the world
        let flash = self.hit_flash / HIT_FLASH_DUR;
        let sway = (self.anim_t * 0.6).sin() * 0.025;
        let attack_lean = if self.state == State::Attack {
            (self.timer / ATTACK_DUR * std::f32::consts::PI).sin() * 0.10
        } else {
            0.0
        };
        let pitch = attack_lean - flash * 0.18;
        let rot = Mat4::from_rotation_y(self.facing)
            * Mat4::from_rotation_z(sway)
            * Mat4::from_rotation_x(pitch);
        let model = Mat4::from_translation(self.pos) * rot;
        for v in &local {
            out.push(Vertex {
                pos: model.transform_point3(Vec3::from(v.pos)).to_array(),
                normal: rot.transform_vector3(Vec3::from(v.normal)).to_array(),
                color: flash_color(v.color, flash * 0.85),
            });
        }
    }
}

/// One branch-arm — upper, lower, fist — rotated about the shoulder.
fn push_warden_arm(local: &mut Vec<Vertex>, x: f32, rot: Mat4, bark: [f32; 3], dark: [f32; 3]) {
    let mut arm: Vec<Vertex> = Vec::new();
    push_box(&mut arm, Vec3::new(x, 3.10, 0.0), Vec3::new(0.28, 0.85, 0.28), bark);
    push_box(&mut arm, Vec3::new(x, 1.70, 0.0), Vec3::new(0.23, 0.78, 0.23), bark);
    push_box(&mut arm, Vec3::new(x, 0.95, 0.0), Vec3::new(0.30, 0.26, 0.26), dark);
    push_rotated(local, &arm, Vec3::new(x, 3.95, 0.0), rot);
}
