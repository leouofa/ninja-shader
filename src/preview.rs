use crate::gpu::engine::GpuEngine;
use crate::pipeline_config::PipelineConfig;
use crate::video::{VideoDecoder, VideoMetadata, VideoProbe};
use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const BLIT_SHADER_WGSL: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index & 1u) * 4 - 1);
    let y = f32(i32((in_vertex_index >> 1u) & 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

@group(0) @binding(0) var blit_sampler: sampler;
@group(0) @binding(1) var blit_texture: texture_2d<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(blit_texture, blit_sampler, in.uv);
}
"#;

struct PreviewApp {
    pipeline_path: PathBuf,
    video_path: PathBuf,
    shaders_dirs: Vec<PathBuf>,
    meta: VideoMetadata,
    render_width: u32,
    render_height: u32,

    // File watcher
    _watcher: Option<RecommendedWatcher>,
    watch_rx: Option<Receiver<notify::Result<Event>>>,

    // Winit & WGPU
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    engine: Option<GpuEngine>,

    // Blit to surface pipeline
    blit_pipeline: Option<wgpu::RenderPipeline>,
    blit_bind_group_layout: Option<wgpu::BindGroupLayout>,
    blit_sampler: Option<wgpu::Sampler>,

    // Video playback
    decoder: Option<VideoDecoder>,
    frame_buffer: Vec<u8>,
    current_frame: u32,
    paused: bool,
    last_frame_instant: Instant,
    frame_duration: Duration,
}

impl PreviewApp {
    fn new(
        pipeline_path: PathBuf,
        video_path: PathBuf,
        shaders_dirs: Vec<PathBuf>,
        scale: f32,
    ) -> Result<Self> {
        let meta = VideoProbe::probe(&video_path)?;
        let render_width = ((meta.width as f32 * scale).round() as u32).max(2) & !1;
        let render_height = ((meta.height as f32 * scale).round() as u32).max(2) & !1;
        let frame_duration = Duration::from_secs_f32(if meta.fps > 0.0 { 1.0 / meta.fps } else { 1.0 / 30.0 });

        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        // Watch pipeline file
        if let Some(parent) = pipeline_path.parent() {
            let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
        }
        // Watch shader dirs
        for dir in &shaders_dirs {
            if dir.exists() {
                let _ = watcher.watch(dir, RecursiveMode::Recursive);
            }
        }

        let decoder = VideoDecoder::spawn(&video_path, render_width, render_height, None, None)?;

        Ok(Self {
            pipeline_path,
            video_path,
            shaders_dirs,
            meta,
            render_width,
            render_height,
            _watcher: Some(watcher),
            watch_rx: Some(rx),
            window: None,
            surface: None,
            surface_config: None,
            engine: None,
            blit_pipeline: None,
            blit_bind_group_layout: None,
            blit_sampler: None,
            decoder: Some(decoder),
            frame_buffer: Vec::new(),
            current_frame: 0,
            paused: false,
            last_frame_instant: Instant::now(),
            frame_duration,
        })
    }

    fn check_hot_reload(&mut self) {
        if let Some(ref rx) = self.watch_rx {
            let mut reload_needed = false;
            while let Ok(event_res) = rx.try_recv() {
                if let Ok(event) = event_res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        reload_needed = true;
                    }
                }
            }

            if reload_needed {
                self.reload_pipeline();
            }
        }
    }

    fn reload_pipeline(&mut self) {
        println!("\x1b[33m[ninja-shader] Hot-reload triggered...\x1b[0m");
        match PipelineConfig::load_from_file(&self.pipeline_path) {
            Ok(new_config) => {
                if let Some(ref mut engine) = self.engine {
                    match engine.build_passes(&new_config, &self.shaders_dirs) {
                        Ok(_) => println!("\x1b[32m[ninja-shader] Pipeline successfully reloaded!\x1b[0m"),
                        Err(e) => eprintln!("\x1b[31m[ninja-shader] Error compiling shaders: {:?}\x1b[0m", e),
                    }
                }
            }
            Err(e) => eprintln!("\x1b[31m[ninja-shader] Error parsing pipeline config: {:?}\x1b[0m", e),
        }
    }

    fn restart_video(&mut self) {
        self.current_frame = 0;
        self.decoder = VideoDecoder::spawn(
            &self.video_path,
            self.render_width,
            self.render_height,
            None,
            None,
        ).ok();
        println!("[ninja-shader] Playback restarted");
    }

    fn save_current_frame(&mut self) {
        if let Some(ref engine) = self.engine {
            let timestamp = chrono_timestamp();
            let out_name = format!("screenshot_{}.png", timestamp);
            let mut rgba = Vec::new();
            if let Ok(_) = engine.readback_frame(&engine.ping_texture, &mut rgba) {
                if let Some(img) = image::RgbaImage::from_raw(self.render_width, self.render_height, rgba) {
                    if img.save(&out_name).is_ok() {
                        println!("\x1b[32m[ninja-shader] Saved screenshot to {}\x1b[0m", out_name);
                    }
                }
            }
        }
    }
}

fn chrono_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title(format!("ninja-shader preview - {}", self.pipeline_path.display()))
            .with_inner_size(winit::dpi::LogicalSize::new(self.render_width as f64, self.render_height as f64));

        let window = Arc::new(event_loop.create_window(window_attrs).expect("Failed to create window"));
        self.window = Some(window.clone());

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).expect("Failed to create wgpu surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })).expect("Failed to get surface compatible adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("preview_device"),
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )).expect("Failed to create device");

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.render_width,
            height: self.render_height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Load pipeline config
        let pipeline_config = PipelineConfig::load_from_file(&self.pipeline_path)
            .expect("Failed to load pipeline config");

        // Init GPU engine
        let engine = GpuEngine::init_with_device(
            instance,
            adapter,
            device.clone(),
            queue.clone(),
            self.render_width,
            self.render_height,
            &pipeline_config,
            &self.shaders_dirs,
        ).expect("Failed to initialize GPU engine");

        // Create blit pipeline
        let blit_sm = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit_shader"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER_WGSL.into()),
        });

        let blit_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit_pipeline_layout"),
            bind_group_layouts: &[&blit_bg_layout],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit_pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_sm,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_sm,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        self.surface = Some(surface);
        self.surface_config = Some(surface_config);
        self.engine = Some(engine);
        self.blit_pipeline = Some(blit_pipeline);
        self.blit_bind_group_layout = Some(blit_bg_layout);
        self.blit_sampler = Some(blit_sampler);

        println!("\x1b[32m[ninja-shader] Preview window opened. Press Space to pause, R to restart, S for screenshot, Esc to quit.\x1b[0m");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(ref mut config), Some(ref engine)) =
                    (&self.surface, &mut self.surface_config, &self.engine)
                {
                    if new_size.width > 0 && new_size.height > 0 {
                        config.width = new_size.width;
                        config.height = new_size.height;
                        surface.configure(&engine.device, config);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Space) => {
                    self.paused = !self.paused;
                    println!("[ninja-shader] {}", if self.paused { "Paused" } else { "Playing" });
                }
                Key::Character(ref ch) if ch.eq_ignore_ascii_case("r") => {
                    self.restart_video();
                }
                Key::Character(ref ch) if ch.eq_ignore_ascii_case("s") => {
                    self.save_current_frame();
                }
                Key::Named(NamedKey::Escape) => {
                    event_loop.exit();
                }
                Key::Character(ref ch) if ch.eq_ignore_ascii_case("q") => {
                    event_loop.exit();
                }
                _ => {}
            },
            WindowEvent::RedrawRequested => {
                self.check_hot_reload();

                let now = Instant::now();
                let need_advance_frame = !self.paused && (now - self.last_frame_instant >= self.frame_duration);

                if need_advance_frame {
                    self.last_frame_instant = now;
                    let mut has_frame = false;
                    if let Some(ref mut decoder) = self.decoder {
                        match decoder.read_frame(&mut self.frame_buffer) {
                            Ok(true) => {
                                has_frame = true;
                                self.current_frame += 1;
                            }
                            Ok(false) => {
                                // Loop video
                                self.restart_video();
                            }
                            Err(e) => {
                                eprintln!("Decoder error: {:?}", e);
                            }
                        }
                    }

                    if has_frame {
                        if let Some(ref engine) = self.engine {
                            engine.upload_source_frame(&self.frame_buffer);
                            engine.update_globals(self.current_frame, self.meta.fps);
                        }
                    }
                }

                // Render frame
                if let (Some(surface), Some(engine), Some(blit_pipeline), Some(blit_layout), Some(blit_sampler)) = (
                    &self.surface,
                    &self.engine,
                    &self.blit_pipeline,
                    &self.blit_bind_group_layout,
                    &self.blit_sampler,
                ) {
                    let surface_output = match surface.get_current_texture() {
                        Ok(frame) => frame,
                        Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
                            if let Some(ref config) = self.surface_config {
                                surface.configure(&engine.device, config);
                            }
                            return;
                        }
                        Err(e) => {
                            eprintln!("Surface error: {:?}", e);
                            return;
                        }
                    };

                    let surface_view = surface_output.texture.create_view(&wgpu::TextureViewDescriptor::default());

                    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("preview_encoder"),
                    });

                    // Render video shader passes
                    let final_tex = engine.render_frame(&mut encoder);
                    let final_view = final_tex.create_view(&wgpu::TextureViewDescriptor::default());

                    // Blit final output to surface
                    let blit_bg = engine.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("blit_bg"),
                        layout: blit_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Sampler(blit_sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&final_view),
                            },
                        ],
                    });

                    {
                        let mut blit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("blit_pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &surface_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                        blit_pass.set_pipeline(blit_pipeline);
                        blit_pass.set_bind_group(0, &blit_bg, &[]);
                        blit_pass.draw(0..3, 0..1);
                    }

                    engine.queue.submit(Some(encoder.finish()));
                    surface_output.present();
                }

                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

pub fn run_preview(
    pipeline_path: PathBuf,
    video_path: PathBuf,
    shaders_dirs: Vec<PathBuf>,
    scale: f32,
) -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = PreviewApp::new(pipeline_path, video_path, shaders_dirs, scale)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
