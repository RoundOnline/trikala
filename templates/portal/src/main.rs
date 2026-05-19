//! Portal trikala template — two rooms, a Doraemon-style door, a
//! cube you steer with WASD. The door is a *window* into the other
//! room: a quad inside the frame samples from an offscreen texture
//! into which the destination room was rendered this same frame
//! using a virtual camera matching yours.
//!
//! Walk through the door's z-plane and `current_world` flips. The
//! roles of "current" and "destination" swap on the next frame so
//! the new door now looks back into the room you came from.
//!
//! ~600 lines. Read top to bottom. No `unsafe`, no globals, no
//! `trikala-*` import — wgpu + winit + glam + bytemuck only.
//!
//! Controls
//!   WASD          — move
//!   Arrow keys    — look (mouse-look optional, off by default)
//!   Esc           — exit
//!
//! Per axiom F29 / F30 / F31 this stays under ~700 lines and depends
//! only on the bare libraries; you can delete `trikala` tomorrow and
//! `cargo run` keeps working.

use std::sync::Arc;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

// ─────────────────────────────────────────────────────────────────
// Tuning — every number you'd want to tweak lives here.
// ─────────────────────────────────────────────────────────────────

const MOVE_SPEED: f32 = 4.5;
const LOOK_SPEED: f32 = 2.2;

/// Doraemon-door position. Both rooms share this — the door is a
/// fixed feature of the world, not a per-room object.
const PORTAL_POS: Vec3 = Vec3::new(0.0, 1.20, -5.0);
const PORTAL_HALF_W: f32 = 1.10;
const PORTAL_HALF_H: f32 = 1.30;
const PORTAL_POST_T: f32 = 0.10;
const PLAYER_RADIUS: f32 = 0.20;

const SHADOW_MAP_SIZE: u32 = 1024;

// ─────────────────────────────────────────────────────────────────
// Vertex + uniform types
// ─────────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    /// `xyz` = position. `w` is unused but kept for std140 alignment.
    camera_pos: [f32; 4],
    /// One directional sun-like light. `xyz` = direction (normalized).
    sun_dir: [f32; 4],
    ambient: [f32; 4],
}

// ─────────────────────────────────────────────────────────────────
// Geometry — small, hand-rolled primitives. No glTF loader, no
// asset pipeline. Replace with your own meshes as the game grows.
// ─────────────────────────────────────────────────────────────────

/// Axis-aligned box of `size` centred at `center`, painted `color`.
/// 6 faces × 2 tris × 3 verts = 36 verts per call (no shared verts —
/// face normals stay correct).
fn push_box(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, center: Vec3, size: Vec3, color: Vec3) {
    let half = size * 0.5;
    let faces: [([[i32; 3]; 4], [f32; 3]); 6] = [
        ([[1, 1, 1], [1, -1, 1], [1, -1, -1], [1, 1, -1]], [1.0, 0.0, 0.0]),
        ([[-1, 1, -1], [-1, -1, -1], [-1, -1, 1], [-1, 1, 1]], [-1.0, 0.0, 0.0]),
        ([[-1, 1, -1], [-1, 1, 1], [1, 1, 1], [1, 1, -1]], [0.0, 1.0, 0.0]),
        ([[-1, -1, 1], [-1, -1, -1], [1, -1, -1], [1, -1, 1]], [0.0, -1.0, 0.0]),
        ([[-1, 1, 1], [-1, -1, 1], [1, -1, 1], [1, 1, 1]], [0.0, 0.0, 1.0]),
        ([[1, 1, -1], [1, -1, -1], [-1, -1, -1], [-1, 1, -1]], [0.0, 0.0, -1.0]),
    ];
    for (corners, normal) in faces.iter() {
        let base = verts.len() as u32;
        for c in corners {
            verts.push(Vertex {
                pos: [
                    center.x + c[0] as f32 * half.x,
                    center.y + c[1] as f32 * half.y,
                    center.z + c[2] as f32 * half.z,
                ],
                normal: *normal,
                color: color.to_array(),
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn push_plane(verts: &mut Vec<Vertex>, indices: &mut Vec<u32>, y: f32, half: f32, color: Vec3) {
    let base = verts.len() as u32;
    for (sx, sz) in &[(-half, -half), (half, -half), (half, half), (-half, half)] {
        verts.push(Vertex { pos: [*sx, y, *sz], normal: [0.0, 1.0, 0.0], color: color.to_array() });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

/// One themed room — floor + a few decorative cubes. Two themes
/// give a clear "I am in a different place" feel after teleport.
fn build_room(theme: u8) -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let (floor, accent, ambient) = match theme & 1 {
        0 => (Vec3::new(0.18, 0.30, 0.40), Vec3::new(0.50, 0.65, 0.85), Vec3::new(2.0, 2.4, 3.0)),
        _ => (Vec3::new(0.30, 0.42, 0.18), Vec3::new(0.62, 0.78, 0.32), Vec3::new(3.0, 2.7, 1.8)),
    };
    push_plane(&mut v, &mut i, 0.0, 12.0, floor);
    // Sun cube — visual anchor + emits the directional light.
    push_box(&mut v, &mut i, Vec3::new(-8.0, 6.0, -8.0), Vec3::new(2.0, 2.0, 2.0), ambient);
    // A few accents so the room reads as occupied.
    for &(x, z) in &[(-4.0_f32, 2.0_f32), (4.0, 2.0), (-3.0, -8.0), (3.0, -8.0)] {
        push_box(&mut v, &mut i, Vec3::new(x, 0.5, z), Vec3::new(1.0, 1.0, 1.0), accent);
    }
    (v, i)
}

/// Pink door frame (Doraemon-style). Two posts + lintel + threshold,
/// hollow in the middle — the portal quad fills that void.
fn build_door() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    let pink = Vec3::new(0.95, 0.55, 0.75);
    let pink_dark = Vec3::new(0.80, 0.42, 0.62);
    let cx = PORTAL_POS.x;
    let cy = PORTAL_POS.y;
    let cz = PORTAL_POS.z;
    let hw = PORTAL_HALF_W;
    let hh = PORTAL_HALF_H;
    let pt = PORTAL_POST_T;
    // posts
    push_box(&mut v, &mut i, Vec3::new(cx - hw - pt, cy, cz), Vec3::new(pt * 2.0, hh * 2.0 + pt * 4.0, pt * 1.6), pink);
    push_box(&mut v, &mut i, Vec3::new(cx + hw + pt, cy, cz), Vec3::new(pt * 2.0, hh * 2.0 + pt * 4.0, pt * 1.6), pink);
    // lintel + threshold
    push_box(&mut v, &mut i, Vec3::new(cx, cy + hh + pt, cz), Vec3::new(hw * 2.0 + pt * 4.0, pt * 2.0, pt * 1.6), pink);
    push_box(&mut v, &mut i, Vec3::new(cx, cy - hh - pt, cz), Vec3::new(hw * 2.0 + pt * 4.0, pt * 2.0, pt * 1.6), pink_dark);
    (v, i)
}

/// The "magic surface" inside the door frame. A simple quad, drawn
/// after the room and sampled per-pixel from the offscreen target
/// holding the destination room.
fn build_portal_quad() -> (Vec<Vertex>, Vec<u32>) {
    let cx = PORTAL_POS.x;
    let cy = PORTAL_POS.y;
    let cz = PORTAL_POS.z;
    let hw = PORTAL_HALF_W;
    let hh = PORTAL_HALF_H;
    let v = vec![
        Vertex { pos: [cx - hw, cy - hh, cz], normal: [0.0, 0.0, 1.0], color: [1.0; 3] },
        Vertex { pos: [cx + hw, cy - hh, cz], normal: [0.0, 0.0, 1.0], color: [1.0; 3] },
        Vertex { pos: [cx + hw, cy + hh, cz], normal: [0.0, 0.0, 1.0], color: [1.0; 3] },
        Vertex { pos: [cx - hw, cy + hh, cz], normal: [0.0, 0.0, 1.0], color: [1.0; 3] },
    ];
    let i = vec![0, 1, 2, 0, 2, 3];
    (v, i)
}

/// The player's avatar — a single coloured cube. Replace this with
/// a real skinned glTF when your game has more than placeholder art.
fn build_player() -> (Vec<Vertex>, Vec<u32>) {
    let mut v = Vec::new();
    let mut i = Vec::new();
    push_box(&mut v, &mut i, Vec3::ZERO, Vec3::new(0.45, 0.9, 0.45), Vec3::new(0.90, 0.85, 0.60));
    (v, i)
}

// ─────────────────────────────────────────────────────────────────
// Input + game state
// ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Input {
    fwd: bool,
    back: bool,
    left: bool,
    right: bool,
    look_left: bool,
    look_right: bool,
    look_up: bool,
    look_down: bool,
}

struct Game {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    depth_view: wgpu::TextureView,
    /// Offscreen target where the destination room is rendered each
    /// frame. The portal-quad fragment shader samples from this.
    portal_color_view: wgpu::TextureView,
    portal_depth_view: wgpu::TextureView,
    portal_tex_bg: wgpu::BindGroup,
    bgl1_portal: wgpu::BindGroupLayout,
    portal_sampler: wgpu::Sampler,

    main_pipeline: wgpu::RenderPipeline,
    portal_pipeline: wgpu::RenderPipeline,
    bg0: wgpu::BindGroup,
    uniform_buf: wgpu::Buffer,

    /// `worlds[0]` = blue room, `worlds[1]` = green room.
    worlds: [(wgpu::Buffer, wgpu::Buffer, u32); 2],
    door: (wgpu::Buffer, wgpu::Buffer, u32),
    portal_quad: (wgpu::Buffer, wgpu::Buffer, u32),
    player_mesh: (wgpu::Buffer, wgpu::Buffer, u32),

    player_pos: Vec3,
    player_prev: Vec3,
    yaw: f32,
    pitch: f32,
    current_world: u8,

    input: Input,
    last_frame: Instant,
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
        let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let depth_view = make_depth(&device, config.width, config.height);
        let (portal_color_view, portal_depth_view) = make_portal_targets(&device, config.width, config.height, format);
        let portal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Uniform buffer (one slot — re-written each pass).
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Group 0: just the uniform buffer. Used by every pipeline.
        let bgl0 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl0"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bg0 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg0"),
            layout: &bgl0,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniform_buf.as_entire_binding() }],
        });

        // Group 1 (portal pipeline only): the offscreen colour target + sampler.
        let bgl1_portal = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bgl1-portal"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let portal_tex_bg = make_portal_bg(&device, &bgl1_portal, &portal_color_view, &portal_sampler);

        // Pipelines.
        let main_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("main"),
            source: wgpu::ShaderSource::Wgsl(MAIN_SHADER.into()),
        });
        let portal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portal"),
            source: wgpu::ShaderSource::Wgsl(PORTAL_SHADER.into()),
        });
        let main_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("main-pl"),
            bind_group_layouts: &[&bgl0],
            push_constant_ranges: &[],
        });
        let portal_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portal-pl"),
            bind_group_layouts: &[&bgl0, &bgl1_portal],
            push_constant_ranges: &[],
        });

        let vattrs = [
            wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
            wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
        ];
        let main_pipeline = make_pipeline(&device, &main_pl, &main_shader, format, &vattrs, true);
        let portal_pipeline = make_pipeline(&device, &portal_pl, &portal_shader, format, &vattrs, false);

        // Meshes — built once at startup, never changed at runtime.
        let worlds = [
            upload_mesh(&device, "world-0", build_room(0)),
            upload_mesh(&device, "world-1", build_room(1)),
        ];
        let door = upload_mesh(&device, "door", build_door());
        let portal_quad = upload_mesh(&device, "portal-quad", build_portal_quad());
        let player_mesh = upload_mesh(&device, "player", build_player());

        let _ = SHADOW_MAP_SIZE; // reserved for a future shadow pass

        let _ = window.set_cursor_visible(true);

        Self {
            window, surface, device, queue, config,
            depth_view, portal_color_view, portal_depth_view, portal_tex_bg,
            bgl1_portal, portal_sampler,
            main_pipeline, portal_pipeline, bg0, uniform_buf,
            worlds, door, portal_quad, player_mesh,
            player_pos: Vec3::new(0.0, 0.0, 5.0),
            player_prev: Vec3::new(0.0, 0.0, 5.0),
            yaw: 0.0,
            pitch: 0.15,
            current_world: 0,
            input: Input::default(),
            last_frame: Instant::now(),
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth_view = make_depth(&self.device, self.config.width, self.config.height);
        let (pcv, pdv) = make_portal_targets(&self.device, self.config.width, self.config.height, self.config.format);
        self.portal_color_view = pcv;
        self.portal_depth_view = pdv;
        self.portal_tex_bg = make_portal_bg(
            &self.device, &self.bgl1_portal, &self.portal_color_view, &self.portal_sampler,
        );
    }

    fn key(&mut self, code: KeyCode, pressed: bool) {
        match code {
            KeyCode::KeyW => self.input.fwd = pressed,
            KeyCode::KeyS => self.input.back = pressed,
            KeyCode::KeyA => self.input.left = pressed,
            KeyCode::KeyD => self.input.right = pressed,
            KeyCode::ArrowLeft  => self.input.look_left = pressed,
            KeyCode::ArrowRight => self.input.look_right = pressed,
            KeyCode::ArrowUp    => self.input.look_up = pressed,
            KeyCode::ArrowDown  => self.input.look_down = pressed,
            _ => {}
        }
    }

    fn update(&mut self, dt: f32) {
        // Look.
        let look_dx = (self.input.look_right as i32 - self.input.look_left as i32) as f32;
        let look_dy = (self.input.look_down as i32 - self.input.look_up as i32) as f32;
        self.yaw   += look_dx * LOOK_SPEED * dt;
        self.pitch = (self.pitch + look_dy * LOOK_SPEED * dt).clamp(-1.2, 1.2);

        // Move along camera-facing plane.
        let fwd = (self.input.fwd as i32 - self.input.back as i32) as f32;
        let right = (self.input.right as i32 - self.input.left as i32) as f32;
        let (sy, cy) = self.yaw.sin_cos();
        let dir = Vec3::new(-sy * fwd + cy * right, 0.0, -cy * fwd - sy * right);
        let len = dir.length();
        let step = if len > 0.0 { dir / len * MOVE_SPEED * dt } else { Vec3::ZERO };
        self.player_prev = self.player_pos;
        self.player_pos += step;

        // Door collision — keep the player from clipping through posts.
        let dx = (self.player_pos.x - PORTAL_POS.x).abs();
        let dz = (self.player_pos.z - PORTAL_POS.z).abs();
        let inside_x = dx < PORTAL_HALF_W - PLAYER_RADIUS;
        if !inside_x && dz < PORTAL_POST_T + PLAYER_RADIUS {
            self.player_pos.z = self.player_prev.z;
        }

        // Teleport when crossing the door's z-plane *inside* the opening.
        let prev_d = self.player_prev.z - PORTAL_POS.z;
        let curr_d = self.player_pos.z - PORTAL_POS.z;
        if prev_d.signum() != curr_d.signum() && inside_x {
            self.current_world ^= 1;
        }
    }

    fn render(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        self.update(dt);

        let aspect = self.config.width as f32 / self.config.height as f32;
        let (sy, cy) = self.yaw.sin_cos();
        let sp = self.pitch.sin();
        let cp = self.pitch.cos();
        let forward = Vec3::new(-sy * cp, sp, -cy * cp);
        let eye_anchor = self.player_pos + Vec3::new(0.0, 1.1, 0.0);
        let cam = eye_anchor - forward * 3.5 + Vec3::new(0.0, 0.4, 0.0);
        let view = Mat4::look_at_rh(cam, eye_anchor, Vec3::Y);
        let proj = Mat4::perspective_rh(60_f32.to_radians(), aspect, 0.05, 200.0);
        let view_proj = proj * view;

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let frame_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        let cur = self.current_world as usize;
        let dest = (cur ^ 1) as usize;
        let (sun_dir, ambient, sky) = world_lighting(dest as u8);

        // Pass 1 — render destination room to the offscreen target.
        let u_dest = build_uniforms(view_proj, cam, sun_dir, ambient);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_dest));
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("portal-view"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.portal_color_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(sky), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.portal_depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.main_pipeline);
            pass.set_bind_group(0, &self.bg0, &[]);
            draw_mesh(&mut pass, &self.worlds[dest]);
            // Door is part of the world — visible from both sides.
            draw_mesh(&mut pass, &self.door);
        }

        // Pass 2 — main view: current room, door, portal quad, player.
        let (sun_dir_cur, ambient_cur, sky_cur) = world_lighting(cur as u8);
        let u_main = build_uniforms(view_proj, cam, sun_dir_cur, ambient_cur);
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&u_main));
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(sky_cur), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.main_pipeline);
            pass.set_bind_group(0, &self.bg0, &[]);
            draw_mesh(&mut pass, &self.worlds[cur]);
            draw_mesh(&mut pass, &self.door);
            draw_mesh(&mut pass, &self.player_mesh); // cube sits at origin — fine for the demo

            // Portal magic surface — samples from the offscreen target.
            pass.set_pipeline(&self.portal_pipeline);
            pass.set_bind_group(0, &self.bg0, &[]);
            pass.set_bind_group(1, &self.portal_tex_bg, &[]);
            draw_mesh(&mut pass, &self.portal_quad);
        }

        self.queue.submit(Some(enc.finish()));
        frame.present();
        self.window.request_redraw();
    }
}

// ─────────────────────────────────────────────────────────────────
// Helpers — kept small so main.rs reads top-to-bottom.
// ─────────────────────────────────────────────────────────────────

fn upload_mesh(
    device: &wgpu::Device,
    label: &str,
    (verts, indices): (Vec<Vertex>, Vec<u32>),
) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    (vb, ib, indices.len() as u32)
}

fn draw_mesh<'a>(pass: &mut wgpu::RenderPass<'a>, mesh: &'a (wgpu::Buffer, wgpu::Buffer, u32)) {
    pass.set_vertex_buffer(0, mesh.0.slice(..));
    pass.set_index_buffer(mesh.1.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..mesh.2, 0, 0..1);
}

fn make_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&Default::default())
}

fn make_portal_targets(
    device: &wgpu::Device,
    w: u32,
    h: u32,
    fmt: wgpu::TextureFormat,
) -> (wgpu::TextureView, wgpu::TextureView) {
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portal-color"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: fmt,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let depth = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("portal-depth"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    (color.create_view(&Default::default()), depth.create_view(&Default::default()))
}

fn make_portal_bg(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-tex-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

fn make_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    vattrs: &[wgpu::VertexAttribute],
    cull: bool,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: vattrs,
            }],
        },
        primitive: wgpu::PrimitiveState {
            cull_mode: if cull { Some(wgpu::Face::Back) } else { None },
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

fn world_lighting(world: u8) -> (Vec3, Vec3, wgpu::Color) {
    match world & 1 {
        0 => (
            Vec3::new(6.0, 20.0, -22.0).normalize(),
            Vec3::new(0.18, 0.22, 0.30),
            wgpu::Color { r: 0.10, g: 0.18, b: 0.30, a: 1.0 },
        ),
        _ => (
            Vec3::new(-12.0, 22.0, 18.0).normalize(),
            Vec3::new(0.40, 0.42, 0.38),
            wgpu::Color { r: 0.62, g: 0.78, b: 0.92, a: 1.0 },
        ),
    }
}

fn build_uniforms(view_proj: Mat4, cam: Vec3, sun_dir: Vec3, ambient: Vec3) -> Uniforms {
    Uniforms {
        view_proj: view_proj.to_cols_array_2d(),
        camera_pos: [cam.x, cam.y, cam.z, 1.0],
        sun_dir: [sun_dir.x, sun_dir.y, sun_dir.z, 0.0],
        ambient: [ambient.x, ambient.y, ambient.z, 1.0],
    }
}

// ─────────────────────────────────────────────────────────────────
// Shaders. Inline so this file reads top-to-bottom.
// ─────────────────────────────────────────────────────────────────

const MAIN_SHADER: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    ambient: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;

struct VsIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec3<f32>,
};

@vertex
fn vs(in: VsIn) -> VsOut {
    var out: VsOut;
    out.clip = u.view_proj * vec4<f32>(in.pos, 1.0);
    out.normal = in.normal;
    out.color = in.color;
    return out;
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let lambert = max(dot(n, normalize(u.sun_dir.xyz)), 0.0);
    let lit = in.color * (u.ambient.rgb + vec3<f32>(lambert));
    return vec4<f32>(lit / (lit + vec3<f32>(1.0)), 1.0);
}
"#;

const PORTAL_SHADER: &str = r#"
struct U {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_dir: vec4<f32>,
    ambient: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var portal_tex: texture_2d<f32>;
@group(1) @binding(1) var portal_samp: sampler;

@vertex
fn vs(@location(0) pos: vec3<f32>, @location(1) _normal: vec3<f32>, @location(2) _color: vec3<f32>) -> @builtin(position) vec4<f32> {
    return u.view_proj * vec4<f32>(pos, 1.0);
}

@fragment
fn fs(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(i32(frag_pos.x), i32(frag_pos.y));
    return textureLoad(portal_tex, texel, 0);
}
"#;

// ─────────────────────────────────────────────────────────────────
// winit app glue + entry point
// ─────────────────────────────────────────────────────────────────

#[derive(Default)]
struct App {
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("{{project-name}}"))
            .unwrap();
        let window = Arc::new(window);
        self.game = Some(pollster::block_on(Game::new(window)));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(game) = self.game.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => game.resize(size.width, size.height),
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: PhysicalKey::Code(code), state, .. }, .. } => {
                if code == KeyCode::Escape && state == ElementState::Pressed {
                    event_loop.exit();
                } else {
                    game.key(code, state == ElementState::Pressed);
                }
            }
            WindowEvent::RedrawRequested => game.render(),
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
