//! Two small monsters that roam the meadow — the quick Sprite and the
//! slow Lurker. Each idles until the player draws near, then closes
//! in, winds up a telegraphed strike, and lashes out. A couple of
//! weapon hits sees one off; slain ones revive after a short delay.

use crate::geometry::{flash_color, push_box, push_box_emissive, push_disc, push_quad, Vertex, FLAT_NORMAL};
use crate::world::tile_height;
use glam::{Mat4, Vec3};

const FLASH_DUR: f32 = 0.18;
const RESPAWN_DELAY: f32 = 5.0; // time a slain monster stays down before reviving
const ATTACK_RANGE: f32 = 1.9; // how close the player must be to start a windup
const ATTACK_RADIUS: f32 = 1.3; // size of the telegraphed strike zone
const RECOVER_DUR: f32 = 0.55; // pause after a strike before chasing again

#[derive(Copy, Clone, PartialEq)]
pub enum MonsterKind {
    Sprite,
    Lurker,
}

impl MonsterKind {
    fn speed(self) -> f32 {
        match self {
            MonsterKind::Sprite => 3.4,
            MonsterKind::Lurker => 1.5,
        }
    }
    fn aggro(self) -> f32 {
        match self {
            MonsterKind::Sprite => 8.5,
            MonsterKind::Lurker => 6.0,
        }
    }
    fn max_hp(self) -> f32 {
        match self {
            MonsterKind::Sprite => 1.5,
            MonsterKind::Lurker => 3.5,
        }
    }
    /// Telegraph length — how long the strike zone shows before the hit.
    fn windup(self) -> f32 {
        match self {
            MonsterKind::Sprite => 0.5,
            MonsterKind::Lurker => 0.85,
        }
    }
}

/// What the monster is doing right now.
#[derive(Copy, Clone, PartialEq)]
enum Mood {
    Roam,
    Windup,
    Recover,
}

pub struct Monster {
    pub kind: MonsterKind,
    pos: Vec3,
    home: Vec3,
    facing: f32,
    hp: f32,
    hit_flash: f32,
    anim_t: f32,
    dead_t: f32,
    mood: Mood,
    attack_t: f32,
    attack_center: Vec3,
    pub alive: bool,
}

impl Monster {
    pub fn new(kind: MonsterKind, x: f32, z: f32) -> Self {
        let pos = Vec3::new(x, tile_height(x.floor() as i32, z.floor() as i32), z);
        Self {
            kind,
            pos,
            home: pos,
            facing: 0.0,
            hp: kind.max_hp(),
            hit_flash: 0.0,
            anim_t: 0.0,
            dead_t: 0.0,
            mood: Mood::Roam,
            attack_t: 0.0,
            attack_center: Vec3::ZERO,
            alive: true,
        }
    }

    pub fn pos(&self) -> Vec3 {
        self.pos
    }

    /// Advance the monster. Returns true on the single frame its
    /// telegraphed strike lands on the player.
    pub fn update(&mut self, dt: f32, player: Vec3) -> bool {
        if !self.alive {
            // count down, then revive at full health back home
            self.dead_t -= dt;
            if self.dead_t <= 0.0 {
                self.alive = true;
                self.hp = self.kind.max_hp();
                self.pos = self.home;
                self.hit_flash = 0.0;
                self.mood = Mood::Roam;
            }
            return false;
        }
        self.anim_t += dt;
        self.hit_flash = (self.hit_flash - dt).max(0.0);
        let to = Vec3::new(player.x - self.pos.x, 0.0, player.z - self.pos.z);
        let dist = to.length();
        let mut struck = false;
        match self.mood {
            Mood::Roam => {
                if dist < self.kind.aggro() && dist > 0.01 {
                    self.facing = to.x.atan2(to.z);
                    if dist < ATTACK_RANGE {
                        // close enough — wind up, locking the strike spot
                        self.mood = Mood::Windup;
                        self.attack_t = 0.0;
                        self.attack_center = player;
                    } else {
                        let step = self.kind.speed() * dt;
                        self.pos.x += to.x / dist * step;
                        self.pos.z += to.z / dist * step;
                    }
                }
            }
            Mood::Windup => {
                if dist > 0.01 {
                    self.facing = to.x.atan2(to.z);
                }
                let prev = self.attack_t;
                self.attack_t += dt;
                let w = self.kind.windup();
                if prev < w && self.attack_t >= w {
                    // the strike lands — does it catch the player?
                    let dx = player.x - self.attack_center.x;
                    let dz = player.z - self.attack_center.z;
                    struck = (dx * dx + dz * dz).sqrt() < ATTACK_RADIUS;
                    self.mood = Mood::Recover;
                    self.attack_t = 0.0;
                }
            }
            Mood::Recover => {
                self.attack_t += dt;
                if self.attack_t >= RECOVER_DUR {
                    self.mood = Mood::Roam;
                }
            }
        }
        self.pos.y = tile_height(self.pos.x.floor() as i32, self.pos.z.floor() as i32);
        struck
    }

    /// Take a weapon hit; returns true if this blow finishes it off.
    pub fn take_hit(&mut self, dmg: f32) -> bool {
        if !self.alive {
            return false;
        }
        self.hp -= dmg;
        self.hit_flash = FLASH_DUR;
        if self.hp <= 0.0 {
            self.alive = false;
            self.dead_t = RESPAWN_DELAY;
        }
        !self.alive
    }

    /// Append the posed monster — and its strike telegraph — to `out`.
    pub fn mesh(&self, out: &mut Vec<Vertex>) {
        if !self.alive {
            return;
        }
        // telegraph — the strike zone, with a progress bar counting down
        // exactly to the moment of the hit
        if self.mood == Mood::Windup {
            let wp = (self.attack_t / self.kind.windup()).clamp(0.0, 1.0);
            let c = self.attack_center;
            let gy = tile_height(c.x.floor() as i32, c.z.floor() as i32);
            // the danger circle
            push_disc(
                out,
                Vec3::new(c.x, gy + 0.05, c.z),
                ATTACK_RADIUS,
                [0.52 + 0.30 * wp, 0.14, 0.11],
            );
            // a flat progress bar across the zone — full = the strike lands
            let (bw, bd) = (1.55, 0.15);
            let bar = |out: &mut Vec<Vertex>, x1: f32, y: f32, col: [f32; 3]| {
                push_quad(
                    out,
                    [
                        Vec3::new(c.x - bw, y, c.z - bd),
                        Vec3::new(x1, y, c.z - bd),
                        Vec3::new(x1, y, c.z + bd),
                        Vec3::new(c.x - bw, y, c.z + bd),
                    ],
                    FLAT_NORMAL,
                    col,
                );
            };
            bar(out, c.x + bw, gy + 0.08, [0.05, 0.04, 0.06]);
            bar(out, c.x - bw + 2.0 * bw * wp, gy + 0.09, [1.0, 0.93, 0.55]);
        }
        let flash = self.hit_flash / FLASH_DUR;
        let bob = (self.anim_t * 6.0).sin() * 0.04;
        let mut local: Vec<Vertex> = Vec::new();
        match self.kind {
            MonsterKind::Sprite => {
                let body = [0.60, 0.40, 0.66];
                push_box(&mut local, Vec3::new(0.0, 0.27 + bob, 0.0), Vec3::new(0.22, 0.22, 0.22), body);
                for ex in [-0.09, 0.09] {
                    push_box_emissive(&mut local, Vec3::new(ex, 0.33 + bob, 0.20), Vec3::new(0.05, 0.06, 0.04), [1.0, 0.95, 0.6]);
                }
                for fx in [-0.12, 0.12] {
                    push_box(&mut local, Vec3::new(fx, 0.05, 0.0), Vec3::new(0.07, 0.05, 0.09), [0.30, 0.22, 0.34]);
                }
            }
            MonsterKind::Lurker => {
                let body = [0.33, 0.41, 0.30];
                push_box(&mut local, Vec3::new(0.0, 0.42 + bob * 0.4, 0.0), Vec3::new(0.44, 0.40, 0.42), body);
                push_box(&mut local, Vec3::new(0.0, 0.80 + bob * 0.4, 0.06), Vec3::new(0.26, 0.22, 0.26), body);
                for ex in [-0.13, 0.13] {
                    push_box_emissive(&mut local, Vec3::new(ex, 0.84 + bob * 0.4, 0.30), Vec3::new(0.06, 0.07, 0.05), [0.96, 0.56, 0.42]);
                }
                for fx in [-0.26, 0.26] {
                    push_box(&mut local, Vec3::new(fx, 0.09, 0.0), Vec3::new(0.14, 0.09, 0.17), [0.24, 0.30, 0.22]);
                }
            }
        }
        let rot = Mat4::from_rotation_y(self.facing);
        let model = Mat4::from_translation(self.pos) * rot;
        for v in &local {
            out.push(Vertex {
                pos: model.transform_point3(Vec3::from(v.pos)).to_array(),
                normal: rot.transform_vector3(Vec3::from(v.normal)).to_array(),
                color: flash_color(v.color, flash * 0.8),
            });
        }
    }
}
