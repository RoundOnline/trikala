use crate::geometry::GrassVertex;
use crate::pipelines::{depth_opaque, make_layout, make_shader};

const GRASS_SHADER: &str = include_str!("../shaders/grass.wgsl");

pub struct GrassSystem {
    pub pipeline: wgpu::RenderPipeline,
    pub vbuf: wgpu::Buffer,
    pub ibuf: wgpu::Buffer,
    pub index_count: u32,
}

pub fn build_grass_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl0: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = make_shader(device, "grass-shader", GRASS_SHADER);
    let layout = make_layout(device, "grass-pl", &[bgl0]);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("grass-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader, entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GrassVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset: 0,  shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 24, shader_location: 3, format: wgpu::VertexFormat::Float32 },
                ],
            }],
        },
        // No culling — blades are paper-thin and viewable from both sides.
        primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
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
