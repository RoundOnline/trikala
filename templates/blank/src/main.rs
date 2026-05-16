//! Blank trikala template — a window that clears to a cycling color.
//!
//! This is a *complete*, self-contained Rust + wgpu + winit program.
//! It does not import from `trikala-*`. You can delete `trikala`
//! tomorrow and this file will still compile.
//!
//! ~150 lines. Read top to bottom; nothing happens off-screen.
//! Per axiom F29 every trikala template must stay under 300 lines.

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

/// All the game state lives in one struct, owned by the loop.
/// No globals. No singletons. Per axiom F5.
struct Game {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    /// Seconds since start — used to cycle the clear color.
    elapsed: f32,
    last_frame: std::time::Instant,
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
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);
        Self {
            window,
            surface,
            device,
            queue,
            config,
            elapsed: 0.0,
            last_frame: std::time::Instant::now(),
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    fn render(&mut self) {
        let now = std::time::Instant::now();
        self.elapsed += now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = self.device.create_command_encoder(&Default::default());

        // The whole "game" — three sine waves, one per channel, cycling.
        // wgpu::Color fields are f64; cast once at the top.
        let t = f64::from(self.elapsed);
        let color = wgpu::Color {
            r: (t * 0.3).sin().mul_add(0.5, 0.5),
            g: (t * 0.5).sin().mul_add(0.5, 0.5),
            b: (t * 0.7).sin().mul_add(0.5, 0.5),
            a: 1.0,
        };

        {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(color), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
        }

        self.queue.submit(Some(enc.finish()));
        frame.present();
        self.window.request_redraw();
    }
}

/// winit 0.30 ApplicationHandler — one struct, three callbacks.
#[derive(Default)]
struct App {
    game: Option<Game>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        self.game = Some(pollster::block_on(Game::new(window)));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(g) = self.game.as_mut() else { return };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => g.resize(s.width, s.height),
            WindowEvent::RedrawRequested => g.render(),
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
