//! Sand patch + the footprint-decal overlay that lays on top of it.
//! Sand is a flat 2-tri mesh; decals are an alpha-blended mesh rebuilt
//! each frame from the active footprint queue.

use glam::Vec3;
use crate::geometry::{DecalVertex, PosVertex};
use crate::pipelines::{depth_opaque, make_layout, make_shader};

const SAND_SHADER:  &str = include_str!("../shaders/sand.wgsl");
const DECAL_SHADER: &str = include_str!("../shaders/decal.wgsl");

pub struct SandSystem {
    pub pipeline: wgpu::RenderPipeline,
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
}

pub struct DecalSystem {
    pub pipeline: wgpu::RenderPipeline,
    /// Footprint mesh rebuilt every frame from `footprints`.
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
    pub vbuf_capacity: usize,
    /// (world position, body_yaw, spawn_time, left_or_right).
    pub footprints: std::collections::VecDeque<(Vec3, f32, f32, f32)>,
    /// Time of the last footprint spawn, so we space them evenly.
    pub last_footprint_t: f32,
}

pub fn build_sand_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "sand-shader", SAND_SHADER);
    let layout = make_layout(device, "sand-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sand-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<PosVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(depth_opaque()),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}

pub fn build_decal_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "decal-shader", DECAL_SHADER);
    let layout = make_layout(device, "decal-pl", &[bgl0]);
    // Decals lie just above the sand — read depth so they don't draw
    // through other geometry, but don't write back (so successive
    // decals can blend without z-fighting).
    let depth = wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: Default::default(),
        bias: wgpu::DepthBiasState { constant: -2, slope_scale: -1.0, clamp: 0.0 },
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("decal-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<DecalVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 20, shader_location: 2, format: wgpu::VertexFormat::Float32 },
                ],
            }],
        },
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
        depth_stencil: Some(depth),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader, entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}
