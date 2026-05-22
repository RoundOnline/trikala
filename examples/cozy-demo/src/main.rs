//! cozy-demo — a small 3D world you can walk, jump and fight in.
//!
//! WASD walks a sword-carrying character (it turns to face its heading
//! and swings its limbs in a walk cycle). Space jumps. Left mouse or
//! the J key slashes: a quick tap is a fast chop; hold then release
//! for a charged overhead smash.
//!
//! The world is endless and loops: terrain is streamed one chunk at a
//! time around the player, so crossing a chunk boundary never stalls.

mod boss;
mod character;
mod geometry;
mod grass;
mod hud;
mod monster;
mod weapon;
mod world;

use boss::{Boss, BOSS_RADIUS};
use character::{body_lean, push_character, sword_arm, CharacterPose, Slash, CHARGE_MIN};
use geometry::{push_blob, push_box, push_disc, Vertex};
use grass::push_grass;
use hud::{push_hud, push_minimap};
use monster::{Monster, MonsterKind};
use weapon::{push_weapon_icon, Weapon};
use glam::{Mat4, Vec3};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};
use world::{build_chunk, push_sky, solid_height, tile_height, CHUNK, WORLD};

const MOVE_SPEED: f32 = 5.0;
const JUMP_SPEED: f32 = 9.5;
const GRAVITY: f32 = 22.0;
const CHAR_R: f32 = 0.34; // character footprint radius, for collision
const STRIDE: f32 = 10.0; // walk-cycle speed (radians per second)
const TURN_SPEED: f32 = 14.0; // how fast the character turns to face (rad/s)
const HURT_DUR: f32 = 0.4; // how long the player's flinch lasts
const KB_SPEED: f32 = 6.0; // knockback speed when the boss connects
const HP_PER_HIT: f32 = 0.2; // health the boss's slam takes from the player
const MONSTER_HIT: f32 = 0.15; // health a monster's strike takes from the player
const SLASH_QUICK: f32 = 1.0; // damage of a quick slash
const SLASH_HEAVY: f32 = 2.2; // damage of a charged heavy slash
const FAINT_DUR: f32 = 1.6; // how long the player is down after losing all health
const PICKUP_RANGE: f32 = 1.7; // walk this close to a weapon pickup to equip it
const LOOT_PICKUP_RANGE: f32 = 1.3; // walk this close to a loot gem to collect it
const MAX_LOOT_GEMS: usize = 64; // uncollected-gem cap, so the vertex buffer can't overflow

/// Weapon pickups dotted around the spawn — walk onto one to equip it.
const PICKUPS: [(Weapon, f32, f32); 3] = [
    (Weapon::Sword, 188.0, 192.0),
    (Weapon::Spear, 192.0, 187.0),
    (Weapon::Bow, 196.0, 192.0),
];

const VIEW: i32 = 2; // chunks loaded in each direction around the player
const SLOTS: usize = 25; // (2*VIEW + 1)^2 terrain slots in the vertex buffer
const MAX_CHUNK_VERTS: usize = 9000; // vertex budget for one chunk's mesh
/// Vertex headroom after the terrain slots for the per-frame dynamic
/// geometry — sky, grass, props, the character, the boss, monsters,
/// loot and the HUD. Asserted against in `render`.
const DYNAMIC_VERT_BUDGET: usize = 24000;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

fn camera(target: Vec3, aspect: f32) -> CameraUniform {
    let eye = target + Vec3::new(8.0, 10.0, 8.0);
    let proj = Mat4::perspective_rh(45f32.to_radians(), aspect, 0.1, 500.0);
    let view = Mat4::look_at_rh(eye, target + Vec3::new(0.0, 0.7, 0.0), Vec3::Y);
    CameraUniform { view_proj: (proj * view).to_cols_array_2d() }
}

fn depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// Which movement keys are currently held.
#[derive(Default)]
struct MoveKeys {
    fwd: bool,
    back: bool,
    left: bool,
    right: bool,
}

/// One terrain chunk's fixed region in the vertex buffer.
#[derive(Clone, Copy)]
struct Slot {
    chunk: Option<(i32, i32)>,
    count: u32,
}

struct Game {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    depth: wgpu::TextureView,
    vbuf: wgpu::Buffer,
    cam_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// One vertex-buffer region per loaded terrain chunk.
    slots: Vec<Slot>,
    /// Scratch buffer for this frame's dynamic geometry and HUD.
    dyn_verts: Vec<Vertex>,
    char_pos: Vec3,
    spawn: Vec3,
    facing: f32,
    vel_y: f32,
    grounded: bool,
    jump_queued: bool,
    walk_t: f32,
    walk_blend: f32,
    attack_held: bool,
    charge: f32,
    attack_anim: Option<(Slash, f32)>,
    move_keys: MoveKeys,
    boss: Boss,
    monsters: Vec<Monster>,
    loot_gems: Vec<Vec3>,
    loot: u32,
    player_hurt: f32,
    hp: f32,
    faint: f32,
    kb_vel: Vec3,
    slash_hit_done: bool,
    weapon: Weapon,
    last: std::time::Instant,
    /// Seconds since launch — drives the grass wind sway.
    time: f32,
}

impl Game {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        // The buffer holds SLOTS fixed terrain regions plus a dynamic
        // tail. Chunks are meshed and uploaded one at a time in `render`.
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertices"),
            size: ((SLOTS * MAX_CHUNK_VERTS + DYNAMIC_VERT_BUDGET) * std::mem::size_of::<Vertex>())
                as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let cam_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: cam_buf.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("scene.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        const ATTRS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("scene"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRS,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // HUD overlay pipeline — flat clip-space bars drawn on top of the
        // scene, no camera and no depth test. See hud.wgsl.
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("hud.wgsl").into()),
        });
        let hud_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud"),
            layout: Some(&hud_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ATTRS,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let depth = depth_view(&device, config.width, config.height);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            pipeline,
            hud_pipeline,
            depth,
            vbuf,
            cam_buf,
            bind_group,
            slots: vec![Slot { chunk: None, count: 0 }; SLOTS],
            dyn_verts: Vec::with_capacity(DYNAMIC_VERT_BUDGET),
            char_pos: Vec3::new(192.0, tile_height(192, 192), 192.0),
            spawn: Vec3::new(192.0, tile_height(192, 192), 192.0),
            facing: -2.356, // start facing into the screen
            vel_y: 0.0,
            grounded: true,
            jump_queued: false,
            walk_t: 0.0,
            walk_blend: 0.0,
            attack_held: false,
            charge: 0.0,
            attack_anim: None,
            move_keys: MoveKeys::default(),
            boss: Boss::new(214.0, 204.0),
            monsters: vec![
                Monster::new(MonsterKind::Sprite, 200.0, 197.0),
                Monster::new(MonsterKind::Sprite, 185.0, 201.0),
                Monster::new(MonsterKind::Lurker, 206.0, 210.0),
                Monster::new(MonsterKind::Lurker, 198.0, 214.0),
            ],
            loot_gems: Vec::new(),
            loot: 0,
            player_hurt: 0.0,
            hp: 1.0,
            faint: 0.0,
            kb_vel: Vec3::ZERO,
            slash_hit_done: false,
            weapon: Weapon::Sword,
            last: std::time::Instant::now(),
            time: 0.0,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth = depth_view(&self.device, self.config.width, self.config.height);
    }

    fn attack_press(&mut self) {
        self.attack_held = true;
    }

    /// Release the attack button — fire a quick or a charged heavy slash.
    fn attack_release(&mut self) {
        self.attack_held = false;
        if self.attack_anim.is_none() {
            let kind = if self.charge >= CHARGE_MIN { Slash::Heavy } else { Slash::Quick };
            self.attack_anim = Some((kind, 0.0));
            self.slash_hit_done = false;
        }
        self.charge = 0.0;
    }

    /// Free slots that scrolled out of range, then mesh any chunks now
    /// missing — one per frame on a normal crossing (no stutter), all at
    /// once on a big jump (a world wrap, which is rare).
    fn stream_terrain(&mut self) {
        let vstride = std::mem::size_of::<Vertex>() as u64;
        let pcx = (self.char_pos.x / CHUNK as f32).floor() as i32;
        let pcz = (self.char_pos.z / CHUNK as f32).floor() as i32;
        for slot in &mut self.slots {
            if let Some((cx, cz)) = slot.chunk {
                if (cx - pcx).abs() > VIEW || (cz - pcz).abs() > VIEW {
                    slot.chunk = None;
                    slot.count = 0;
                }
            }
        }
        let mut missing: Vec<(i32, i32)> = Vec::new();
        for dz in -VIEW..=VIEW {
            for dx in -VIEW..=VIEW {
                let coord = (pcx + dx, pcz + dz);
                if !self.slots.iter().any(|s| s.chunk == Some(coord)) {
                    missing.push(coord);
                }
            }
        }
        let build_now = if missing.len() > 9 { missing.len() } else { 1 };
        for &coord in missing.iter().take(build_now) {
            if let Some(si) = self.slots.iter().position(|s| s.chunk.is_none()) {
                let mesh = build_chunk(coord.0, coord.1);
                debug_assert!(mesh.len() <= MAX_CHUNK_VERTS, "chunk exceeded MAX_CHUNK_VERTS");
                self.queue.write_buffer(
                    &self.vbuf,
                    (si * MAX_CHUNK_VERTS) as u64 * vstride,
                    bytemuck::cast_slice(&mesh),
                );
                self.slots[si].chunk = Some(coord);
                self.slots[si].count = mesh.len() as u32;
            }
        }
    }

    fn render(&mut self) {
        let now = std::time::Instant::now();
        let dt = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        self.time += dt;
        self.player_hurt = (self.player_hurt - dt).max(0.0);
        if self.faint > 0.0 {
            self.faint -= dt;
            if self.faint <= 0.0 {
                // revive at the spawn point, fully recovered, with no
                // held input or part-finished attack carried over
                self.hp = 1.0;
                self.char_pos = self.spawn;
                self.vel_y = 0.0;
                self.kb_vel = Vec3::ZERO;
                self.attack_held = false;
                self.charge = 0.0;
                self.attack_anim = None;
            }
        }

        // horizontal movement — camera-relative, so W heads up-screen.
        // Each axis resolves on its own (probe CHAR_R ahead) so the body
        // never sinks into a cliff but can still slide along one.
        let fwd = Vec3::new(-1.0, 0.0, -1.0).normalize();
        let right = Vec3::new(1.0, 0.0, -1.0).normalize();
        let mut mv = Vec3::ZERO;
        if self.move_keys.fwd { mv += fwd; }
        if self.move_keys.back { mv -= fwd; }
        if self.move_keys.right { mv += right; }
        if self.move_keys.left { mv -= right; }
        if self.faint > 0.0 {
            mv = Vec3::ZERO; // knocked out — can't move
        }
        let moving = mv != Vec3::ZERO;
        if moving {
            let step = mv.normalize() * MOVE_SPEED * dt;
            let reach = self.char_pos.y + 0.05;
            if step.x != 0.0 {
                let probe = self.char_pos.x + step.x + step.x.signum() * CHAR_R;
                if reach >= solid_height(probe, self.char_pos.z) {
                    self.char_pos.x += step.x;
                }
            }
            if step.z != 0.0 {
                let probe = self.char_pos.z + step.z + step.z.signum() * CHAR_R;
                if reach >= solid_height(self.char_pos.x, probe) {
                    self.char_pos.z += step.z;
                }
            }
            self.walk_t += dt;
            // turn smoothly to face the heading
            let target = mv.x.atan2(mv.z);
            let mut d = target - self.facing;
            let pi = std::f32::consts::PI;
            while d > pi { d -= std::f32::consts::TAU; }
            while d < -pi { d += std::f32::consts::TAU; }
            self.facing += d.clamp(-TURN_SPEED * dt, TURN_SPEED * dt);
        }

        // knockback from a boss hit, fading out
        if self.kb_vel.length_squared() > 1e-4 {
            self.char_pos.x += self.kb_vel.x * dt;
            self.char_pos.z += self.kb_vel.z * dt;
            self.kb_vel *= (1.0 - dt * 9.0).max(0.0);
        }
        // the boss is solid — never let the player stand inside it
        let bp = self.boss.pos();
        let (dx, dz) = (self.char_pos.x - bp.x, self.char_pos.z - bp.z);
        let d = (dx * dx + dz * dz).sqrt();
        let clear = BOSS_RADIUS + CHAR_R;
        if d > 1e-4 && d < clear {
            self.char_pos.x = bp.x + dx / d * clear;
            self.char_pos.z = bp.z + dz / d * clear;
        }

        // the world loops — wrap the player around it
        self.char_pos.x = self.char_pos.x.rem_euclid(WORLD as f32);
        self.char_pos.z = self.char_pos.z.rem_euclid(WORLD as f32);

        // walk onto a weapon pickup to equip that weapon
        for (w, px, pz) in PICKUPS {
            let (dx, dz) = (self.char_pos.x - px, self.char_pos.z - pz);
            if (dx * dx + dz * dz).sqrt() < PICKUP_RANGE {
                self.weapon = w;
            }
        }

        // jump + gravity
        if self.jump_queued && self.grounded && self.faint <= 0.0 {
            self.vel_y = JUMP_SPEED;
        }
        self.jump_queued = false;
        self.vel_y -= GRAVITY * dt;
        self.char_pos.y += self.vel_y * dt;
        let floor = solid_height(self.char_pos.x, self.char_pos.z);
        if self.char_pos.y <= floor {
            self.char_pos.y = floor;
            self.vel_y = 0.0;
            self.grounded = true;
        } else {
            self.grounded = false;
        }

        // attack timing
        if let Some((_, t)) = self.attack_anim.as_mut() {
            *t += dt;
        }
        if let Some((kind, t)) = self.attack_anim {
            if t >= kind.duration() {
                self.attack_anim = None;
            }
        }
        if self.attack_held && self.attack_anim.is_none() {
            self.charge = (self.charge + dt).min(1.2);
        }

        // a weapon strike connects with the boss and any monsters that
        // are in reach AND in front of the player — resolved once
        if let Some((kind, t)) = self.attack_anim {
            if !self.slash_hit_done && t / kind.duration() >= 0.4 {
                self.slash_hit_done = true;
                let dmg = match kind {
                    Slash::Heavy => SLASH_HEAVY,
                    Slash::Quick => SLASH_QUICK,
                };
                let reach = self.weapon.reach();
                let (fx, fz) = (self.facing.sin(), self.facing.cos());
                let (px, pz) = (self.char_pos.x, self.char_pos.z);
                // connects only when the weapon reaches the target's
                // surface (its radius) and the target is in front
                let hits = |tx: f32, tz: f32, radius: f32| {
                    let (dx, dz) = (tx - px, tz - pz);
                    let d = (dx * dx + dz * dz).sqrt();
                    d > 0.01 && d - radius < reach && (dx * fx + dz * fz) / d > 0.2
                };
                let b = self.boss.pos();
                if hits(b.x, b.z, BOSS_RADIUS) {
                    self.boss.take_hit(dmg);
                }
                for m in &mut self.monsters {
                    let mp = m.pos();
                    if hits(mp.x, mp.z, 0.5)
                        && m.take_hit(dmg)
                        && self.loot_gems.len() < MAX_LOOT_GEMS
                    {
                        self.loot_gems.push(mp);
                    }
                }
            }
        }

        // ease the walk animation in/out; bob the body on each footfall
        let target_blend = if moving && self.grounded { 1.0 } else { 0.0 };
        self.walk_blend += (target_blend - self.walk_blend) * (dt * 10.0).min(1.0);
        let phase = self.walk_t * STRIDE;
        let bob = phase.sin().abs() * 0.05 * self.walk_blend;

        // upload the camera, then stream terrain chunks
        let aspect = self.config.width as f32 / self.config.height.max(1) as f32;
        self.queue.write_buffer(&self.cam_buf, 0, bytemuck::bytes_of(&camera(self.char_pos, aspect)));
        self.stream_terrain();

        // advance the boss; if its slam just landed, test the player
        self.boss.update(dt, self.char_pos);
        if self.boss.struck() {
            if let Some((center, radius)) = self.boss.attack_zone() {
                let (hx, hz) = (self.char_pos.x - center.x, self.char_pos.z - center.z);
                if (hx * hx + hz * hz).sqrt() < radius {
                    self.player_hurt = HURT_DUR;
                    if self.faint <= 0.0 {
                        self.hp = (self.hp - HP_PER_HIT).max(0.0);
                        if self.hp <= 0.0 {
                            self.faint = FAINT_DUR;
                        }
                    }
                    let b = self.boss.pos();
                    let (kx, kz) = (self.char_pos.x - b.x, self.char_pos.z - b.z);
                    let kl = (kx * kx + kz * kz).sqrt().max(0.01);
                    self.kb_vel = Vec3::new(kx / kl * KB_SPEED, 0.0, kz / kl * KB_SPEED);
                }
            }
        }

        // boss defeated — scatter a few loot gems where it fell
        if self.boss.defeated() {
            let b = self.boss.pos();
            for k in 0..3u32 {
                if self.loot_gems.len() >= MAX_LOOT_GEMS {
                    break;
                }
                let a = k as f32 * 2.1;
                self.loot_gems.push(b + Vec3::new(a.cos() * 1.6, 0.0, a.sin() * 1.6));
            }
        }

        // monsters chase, telegraph a strike, then lash out at the player
        for m in &mut self.monsters {
            if m.update(dt, self.char_pos) && self.faint <= 0.0 {
                self.player_hurt = HURT_DUR;
                self.hp = (self.hp - MONSTER_HIT).max(0.0);
                if self.hp <= 0.0 {
                    self.faint = FAINT_DUR;
                }
            }
        }

        // walk over a dropped loot gem to collect it
        let mut gi = 0;
        while gi < self.loot_gems.len() {
            let g = self.loot_gems[gi];
            let (dx, dz) = (self.char_pos.x - g.x, self.char_pos.z - g.z);
            if (dx * dx + dz * dz).sqrt() < LOOT_PICKUP_RANGE {
                self.loot_gems.swap_remove(gi);
                self.loot += 1;
            } else {
                gi += 1;
            }
        }

        // rebuild this frame's dynamic geometry — world props, actors, HUD
        self.dyn_verts.clear();
        push_sky(&mut self.dyn_verts, self.char_pos.x, self.char_pos.z);
        push_grass(&mut self.dyn_verts, self.char_pos, self.time);
        push_disc(
            &mut self.dyn_verts,
            Vec3::new(self.spawn.x, self.spawn.y + 0.04, self.spawn.z),
            0.8,
            [0.92, 0.84, 0.55],
        );
        // weapon pickups — a small pad and a slowly spinning weapon model
        for (w, px, pz) in PICKUPS {
            let gy = tile_height(px as i32, pz as i32);
            push_disc(&mut self.dyn_verts, Vec3::new(px, gy + 0.04, pz), 0.55, [0.55, 0.62, 0.74]);
            let bob = (self.time * 2.0 + px).sin() * 0.06;
            push_weapon_icon(
                &mut self.dyn_verts,
                w,
                Vec3::new(px, gy + 0.7 + bob, pz),
                self.time * 1.4,
            );
        }
        let shadow_at = Vec3::new(
            self.char_pos.x,
            solid_height(self.char_pos.x, self.char_pos.z),
            self.char_pos.z,
        );
        push_blob(&mut self.dyn_verts, shadow_at, 0.42);
        // a steady tint while knocked out, otherwise the brief hit flash
        let hurt = if self.faint > 0.0 { 0.55 } else { self.player_hurt / HURT_DUR };
        let pose = CharacterPose {
            pos: self.char_pos + Vec3::new(0.0, bob, 0.0),
            facing: self.facing,
            phase,
            blend: self.walk_blend,
            sword_arm: sword_arm(self.attack_anim, self.attack_held, self.charge, phase, self.walk_blend),
            lean: body_lean(self.attack_anim) - hurt * 0.4,
            flash: hurt,
            weapon: self.weapon,
        };
        push_character(&mut self.dyn_verts, &pose);
        self.boss.mesh(&mut self.dyn_verts);
        for m in &self.monsters {
            m.mesh(&mut self.dyn_verts);
        }
        // dropped loot — small gold gems that bob in place
        for g in &self.loot_gems {
            let bob = 0.32 + (self.time * 3.0 + g.x).sin() * 0.09;
            push_box(
                &mut self.dyn_verts,
                Vec3::new(g.x, g.y + bob, g.z),
                Vec3::splat(0.12),
                [0.96, 0.81, 0.32],
            );
        }
        // HUD bars go last in the buffer so they draw with their own pipeline
        let hud_start = self.dyn_verts.len();
        push_hud(
            &mut self.dyn_verts,
            self.hp,
            self.boss.hp_fraction(),
            self.boss.awake(),
            self.loot,
        );
        // minimap blips — boss, living monsters, weapon pickups, spawn
        let mut blips: Vec<(Vec3, [f32; 3], f32)> = Vec::new();
        blips.push((self.boss.pos(), [0.86, 0.32, 0.26], 0.022));
        for m in &self.monsters {
            if m.alive {
                let col = match m.kind {
                    MonsterKind::Sprite => [0.66, 0.45, 0.78],
                    MonsterKind::Lurker => [0.45, 0.62, 0.42],
                };
                blips.push((m.pos(), col, 0.014));
            }
        }
        for (_, px, pz) in PICKUPS {
            blips.push((Vec3::new(px, 0.0, pz), [0.55, 0.70, 0.92], 0.013));
        }
        blips.push((self.spawn, [0.92, 0.84, 0.55], 0.015));
        push_minimap(&mut self.dyn_verts, self.char_pos, &blips);
        debug_assert!(
            self.dyn_verts.len() <= DYNAMIC_VERT_BUDGET,
            "dynamic geometry exceeded DYNAMIC_VERT_BUDGET",
        );
        let vstride = std::mem::size_of::<Vertex>() as u64;
        let dyn_base = (SLOTS * MAX_CHUNK_VERTS) as u64 * vstride;
        self.queue
            .write_buffer(&self.vbuf, dyn_base, bytemuck::cast_slice(&self.dyn_verts));

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.5, g: 0.4, b: 0.4, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            // one draw per loaded terrain chunk, then the dynamic tail
            for (i, slot) in self.slots.iter().enumerate() {
                if slot.count > 0 {
                    let base = (i * MAX_CHUNK_VERTS) as u32;
                    pass.draw(base..base + slot.count, 0..1);
                }
            }
            // scene dynamic geometry — sky, grass, character, boss
            let dyn_start = (SLOTS * MAX_CHUNK_VERTS) as u32;
            pass.draw(dyn_start..dyn_start + hud_start as u32, 0..1);
            // HUD bars, drawn on top in clip space by their own pipeline
            pass.set_pipeline(&self.hud_pipeline);
            pass.draw(
                dyn_start + hud_start as u32..dyn_start + self.dyn_verts.len() as u32,
                0..1,
            );
        }
        self.queue.submit(Some(enc.finish()));
        frame.present();
        self.window.request_redraw();
    }
}

#[derive(Default)]
struct App {
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("cozy-demo — WASD move, Space jump, click / J to slash");
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        self.game = Some(pollster::block_on(Game::new(window)));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(g) = self.game.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => g.resize(s.width, s.height),
            WindowEvent::RedrawRequested => g.render(),
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    match state {
                        ElementState::Pressed => g.attack_press(),
                        ElementState::Released => g.attack_release(),
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key: PhysicalKey::Code(code), state, repeat, .. },
                ..
            } => {
                let down = state == ElementState::Pressed;
                match code {
                    KeyCode::KeyW => g.move_keys.fwd = down,
                    KeyCode::KeyA => g.move_keys.left = down,
                    KeyCode::KeyS => g.move_keys.back = down,
                    KeyCode::KeyD => g.move_keys.right = down,
                    KeyCode::Space if down && !repeat => g.jump_queued = true,
                    KeyCode::KeyJ => match (down, repeat) {
                        (true, false) => g.attack_press(),
                        (false, _) => g.attack_release(),
                        _ => {}
                    },
                    KeyCode::Escape => event_loop.exit(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
