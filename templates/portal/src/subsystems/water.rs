use glam::Vec3;
use crate::geometry::PosVertex;
use crate::pipelines::{depth_opaque, make_layout, make_shader};

const WATER_SHADER: &str = include_str!("../shaders/water.wgsl");

pub struct WaterSystem {
    pub pipeline: wgpu::RenderPipeline,
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
    /// Active ripples — (world-space xz, spawn time). Capped at 8.
    pub ripples: std::collections::VecDeque<(Vec3, f32)>,
    /// Packed into the main Uniforms buffer for the water shader.
    pub ripples_uniform: [[f32; 4]; 8],
}

pub fn build_water_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "water-shader", WATER_SHADER);
    let layout = make_layout(device, "water-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("water-pipeline"),
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None, cache: None,
    })
}
