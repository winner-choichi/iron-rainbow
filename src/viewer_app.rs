use std::{env, fs, sync::Arc, time::Instant};

use crate::lut::{FalseColorStop, LutConfig};
use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use chrono::Local;
use log::warn;
use serde::Deserialize;
use wgpu::util::DeviceExt;
use wgpu::{SurfaceConfiguration, SurfaceError};
use winit::dpi::PhysicalSize;
use winit::event::*;
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

const MAX_FALSE_COLOR_STOPS: usize = 16;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct LutMetadata {
    wavelength_min_nm: f32,
    wavelength_max_nm: f32,
    wavelength_values_nm: Vec<f32>,
    angle_min_deg: f32,
    angle_max_deg: f32,
    angle_steps: u32,
    normalize_mode: String,
    exposure: f32,
    false_color: Vec<FalseColorStop>,
    droplet_radius_um: f32,
    rays_per_wavelength: u32,
    #[serde(default)]
    channel_wavelengths: Option<Vec<f32>>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct Droplet {
    position: [f32; 3],
    radius: f32,
}

struct ParticleCloud {
    box_center: [f32; 3],
    box_size: [f32; 3],
    droplet_radius: f32,
    droplets: Vec<Droplet>,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
struct ViewerUniform {
    // Camera (vec3 + padding)
    camera_pos: [f32; 3],
    _pad0: f32,
    camera_forward: [f32; 3],
    _pad1: f32,
    camera_right: [f32; 3],
    _pad2: f32,
    camera_up: [f32; 3],
    _pad3: f32,

    // Sun
    sun_dir: [f32; 3],
    _pad4: f32,

    // Droplet region
    droplet_center: [f32; 3],
    droplet_radius: f32,

    // Viewport
    viewport_width: f32,
    viewport_height: f32,
    fov: f32,
    exposure: f32,

    // LUT params
    wavelength_min: f32,
    wavelength_range: f32,
    angle_min_deg: f32,
    angle_range_deg: f32,

    // Texture and rendering
    tex_width: u32,
    tex_height: u32,
    debug_mode: u32,
    march_steps: u32,

    channel_wavelengths: [f32; 4],
    false_color_count: u32,
    _pad_fc: [u32; 3],
    false_color_data: [[f32; 4]; MAX_FALSE_COLOR_STOPS],
    _pad_end: [f32; 4],
}

struct Camera {
    position: [f32; 3],
    yaw: f32,
    pitch: f32,
    fov: f32,
}

impl Camera {
    fn new(initial_pos: [f32; 3], look_at: [f32; 3]) -> Self {
        let direction = normalize([
            look_at[0] - initial_pos[0],
            look_at[1] - initial_pos[1],
            look_at[2] - initial_pos[2],
        ]);
        let pitch = direction[1].asin();
        let yaw = direction[2].atan2(direction[0]);
        Self {
            position: initial_pos,
            yaw,
            pitch,
            fov: 60.0_f32.to_radians(),
        }
    }

    fn position(&self) -> [f32; 3] {
        self.position
    }

    fn forward(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        let dir = [cp * self.yaw.sin(), self.pitch.sin(), cp * self.yaw.cos()];
        normalize(dir)
    }

    fn right(&self) -> [f32; 3] {
        let forward = self.forward();
        normalize(cross([0.0, 1.0, 0.0], forward))
    }

    fn up(&self) -> [f32; 3] {
        let forward = self.forward();
        let right = self.right();
        normalize(cross(forward, right))
    }

    fn move_local(&mut self, forward_delta: f32, right_delta: f32, up_delta: f32) {
        let forward = self.forward();
        let right = self.right();
        let up = [0.0, 1.0, 0.0];

        self.position[0] += forward[0] * forward_delta + right[0] * right_delta + up[0] * up_delta;
        self.position[1] += forward[1] * forward_delta + right[1] * right_delta + up[1] * up_delta;
        self.position[2] += forward[2] * forward_delta + right[2] * right_delta + up[2] * up_delta;
    }

    fn reset(&mut self, new_pos: [f32; 3], look_at: [f32; 3]) {
        self.position = new_pos;
        let direction = normalize([
            look_at[0] - new_pos[0],
            look_at[1] - new_pos[1],
            look_at[2] - new_pos[2],
        ]);
        self.pitch = direction[1].asin();
        self.yaw = direction[2].atan2(direction[0]);
    }
}

struct ViewerState {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    uniform: ViewerUniform,

    // 3D scene
    camera: Camera,
    initial_camera_pos: [f32; 3],
    initial_look_at: [f32; 3],
    sun_elevation: f32, // degrees
    sun_azimuth: f32,   // degrees

    // Controls
    exposure_multiplier: f32,
    debug_mode: u32,
    march_steps: u32,

    // Movement
    move_speed: f32,
    mouse_sensitivity: f32,
    keys_pressed: std::collections::HashSet<String>,

    // Metadata
    metadata: LutMetadata,
    frame_count: u64,
    last_fps_update: Instant,
    fps: f32,
    last_frame_time: Instant,
}

impl ViewerState {
    async fn new(window: winit::window::Window, lut_config_path: &str) -> Result<Self> {
        let window = Arc::new(window);
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow!("Failed to find suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Viewer Device"),
                    required_features: wgpu::Features::TEXTURE_FORMAT_16BIT_NORM,
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let lut_config = LutConfig::load(lut_config_path)
            .map_err(|e| anyhow!("Failed to load LUT config: {e}"))?;
        let metadata_text = fs::read_to_string(&lut_config.output.metadata_path)
            .map_err(|e| anyhow!("Failed to read LUT metadata: {e}"))?;
        let metadata: LutMetadata = serde_json::from_str(&metadata_text)
            .map_err(|e| anyhow!("Failed to parse LUT metadata: {e}"))?;

        let lut_image = image::ImageReader::open(&lut_config.output.texture_path)
            .map_err(|e| anyhow!("Failed to open LUT texture: {e}"))?
            .decode()
            .map_err(|e| anyhow!("Failed to decode LUT texture: {e}"))?
            .to_luma16();
        let lut_width = lut_image.width();
        let lut_height = lut_image.height();
        let mut lut_bytes = Vec::with_capacity((lut_width as usize) * (lut_height as usize) * 2);
        for value in lut_image.as_raw() {
            lut_bytes.extend_from_slice(&value.to_le_bytes());
        }

        let texture_size = wgpu::Extent3d {
            width: lut_width,
            height: lut_height,
            depth_or_array_layers: 1,
        };

        let lut_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("LUT Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &lut_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &lut_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(lut_width * 2),
                rows_per_image: Some(lut_height),
            },
            texture_size,
        );

        let lut_view = lut_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let lut_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("LUT Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Load planet texture
        let planet_image = image::ImageReader::open("planet_texture.jpg")
            .map_err(|e| anyhow!("Failed to open planet texture: {e}"))?
            .decode()
            .map_err(|e| anyhow!("Failed to decode planet texture: {e}"))?
            .to_rgba8();
        let planet_width = planet_image.width();
        let planet_height = planet_image.height();

        let planet_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Planet Texture"),
            size: wgpu::Extent3d {
                width: planet_width,
                height: planet_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &planet_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            planet_image.as_raw(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(planet_width * 4),
                rows_per_image: Some(planet_height),
            },
            wgpu::Extent3d {
                width: planet_width,
                height: planet_height,
                depth_or_array_layers: 1,
            },
        );

        let planet_view = planet_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let planet_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Planet Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let initial_camera_pos = [0.0, 12000.0, 24000.0];
        let initial_look_at = [0.0, 4000.0, 0.0];
        let camera = Camera::new(initial_camera_pos, initial_look_at);

        // Initialize sun (45 degrees elevation, 135 degrees azimuth)
        let sun_elevation = 5.0;
        let sun_azimuth = 135.0;

        // No particle cloud needed - phase function approach!

        let channel_wavelengths = select_channel_wavelengths(&metadata);
        let (false_color_data, false_color_count) = pack_false_color_data(&metadata);
        let (wavelength_min, wavelength_range) = resolve_wavelength_axis(&metadata, &lut_config);
        let (angle_min_deg, angle_range_deg) = resolve_angle_axis(&metadata, &lut_config);

        let march_steps = 64;

        let uniform = ViewerUniform {
            camera_pos: camera.position(),
            _pad0: 0.0,
            camera_forward: camera.forward(),
            _pad1: 0.0,
            camera_right: camera.right(),
            _pad2: 0.0,
            camera_up: camera.up(),
            _pad3: 0.0,
            sun_dir: sun_direction(sun_elevation, sun_azimuth),
            _pad4: 0.0,
            droplet_center: [0.0, 0.0, 0.0],
            droplet_radius: 10000.0,
            viewport_width: size.width as f32,
            viewport_height: size.height as f32,
            fov: camera.fov,
            exposure: metadata.exposure,
            wavelength_min,
            wavelength_range,
            angle_min_deg,
            angle_range_deg,
            tex_width: lut_width,
            tex_height: lut_height,
            debug_mode: 0,
            march_steps,
            channel_wavelengths: [
                channel_wavelengths[0],
                channel_wavelengths[1],
                channel_wavelengths[2],
                0.0,
            ],
            false_color_count,
            _pad_fc: [0; 3],
            false_color_data,
            _pad_end: [0.0; 4],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Viewer Uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Viewer Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Viewer Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&lut_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&lut_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&planet_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&planet_sampler),
                },
            ],
        });

        // Load rainbow phase function shader
        let shader_source = include_str!("shaders/rainbow_phase.wgsl");
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rainbow Phase Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Viewer Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Viewer Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let now = Instant::now();
        println!("\n=== Iron Rainbow 3D Viewer ===");
        println!("LUT: {}", lut_config.output.texture_path);
        println!(
            "Wavelength: {:.0}-{:.0} nm",
            metadata.wavelength_min_nm, metadata.wavelength_max_nm
        );
        println!(
            "Angle: {:.0}°-{:.0}°",
            metadata.angle_min_deg, metadata.angle_max_deg
        );
        println!("Droplet Radius: 10,000m (visualization scale)");
        println!("\n=== Orbit Camera Controls ===");
        println!("  Mouse Drag - Rotate around droplet");
        println!("  W/S        - Pan up/down");
        println!("  A/D        - Pan left/right");
        println!("  Q/E        - Zoom in/out");
        println!("  R          - Reset camera");
        println!("\n=== Rendering Controls ===");
        println!("  ↑/↓        - Sun elevation");
        println!("  ←/→        - Sun azimuth");
        println!("  +/-        - Exposure");
        println!("  [/]        - March steps");
        println!("  0-9        - Debug modes");
        println!("  P          - Print status");
        println!("  ESC        - Release mouse");
        println!("===============================\n");

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            bind_group,
            uniform_buffer,
            uniform,
            camera,
            initial_camera_pos,
            initial_look_at,
            sun_elevation,
            sun_azimuth,
            exposure_multiplier: 0.01,
            debug_mode: 0,
            march_steps,
            move_speed: 100000.0,
            mouse_sensitivity: 0.002,
            keys_pressed: std::collections::HashSet::new(),
            metadata,
            frame_count: 0,
            last_fps_update: now,
            fps: 0.0,
            last_frame_time: now,
        })
    }

    fn window(&self) -> &winit::window::Window {
        &self.window
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.uniform.viewport_width = new_size.width as f32;
            self.uniform.viewport_height = new_size.height as f32;
        }
    }

    fn update(&mut self) {
        // Update FPS
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_fps_update).as_secs_f32();
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_fps_update = now;
        }

        // Delta time for movement
        let dt = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        let pan_speed = self.move_speed * dt;
        let mut forward_delta = 0.0;
        let mut right_delta = 0.0;
        let mut up_delta = 0.0;

        if self.keys_pressed.contains("w") {
            forward_delta += pan_speed;
        }
        if self.keys_pressed.contains("s") {
            forward_delta -= pan_speed;
        }
        if self.keys_pressed.contains("a") {
            right_delta -= pan_speed;
        }
        if self.keys_pressed.contains("d") {
            right_delta += pan_speed;
        }
        if self.keys_pressed.contains("q") {
            up_delta -= pan_speed;
        }
        if self.keys_pressed.contains("e") {
            up_delta += pan_speed;
        }

        if forward_delta != 0.0 || right_delta != 0.0 || up_delta != 0.0 {
            self.camera.move_local(forward_delta, right_delta, up_delta);
        }

        // Update uniform
        self.uniform.camera_pos = self.camera.position();
        self.uniform.camera_forward = self.camera.forward();
        self.uniform.camera_right = self.camera.right();
        self.uniform.camera_up = self.camera.up();
        self.uniform.sun_dir = sun_direction(self.sun_elevation, self.sun_azimuth);
        self.uniform.exposure = self.metadata.exposure * self.exposure_multiplier;
        self.uniform.march_steps = self.march_steps;

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.uniform));
    }

    fn handle_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        self.camera.yaw += delta_x as f32 * self.mouse_sensitivity;
        self.camera.pitch += delta_y as f32 * self.mouse_sensitivity;
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        self.camera.pitch = self.camera.pitch.clamp(-max_pitch, max_pitch);
    }

    fn print_status(&self) {
        let pos = self.camera.position();
        let forward = self.camera.forward();
        let sun = self.uniform.sun_dir;
        let droplet = self.uniform.droplet_center;

        println!("\n========== Status ==========");
        println!("FPS: {:.1}", self.fps);
        println!("\n[Camera]");
        println!("  Position: [{:.1}, {:.1}, {:.1}]", pos[0], pos[1], pos[2]);
        println!(
            "  Forward:  [{:.3}, {:.3}, {:.3}]",
            forward[0], forward[1], forward[2]
        );
        println!(
            "  Yaw: {:.1}°, Pitch: {:.1}°",
            self.camera.yaw.to_degrees(),
            self.camera.pitch.to_degrees()
        );

        println!("\n[Sun]");
        println!(
            "  Elevation: {:.1}°, Azimuth: {:.1}°",
            self.sun_elevation, self.sun_azimuth
        );
        println!("  Direction: [{:.3}, {:.3}, {:.3}]", sun[0], sun[1], sun[2]);

        println!("\n[Droplet Sphere]");
        println!(
            "  Center: [{:.1}, {:.1}, {:.1}]",
            droplet[0], droplet[1], droplet[2]
        );
        println!("  Radius: {:.1}m", self.uniform.droplet_radius);

        println!("\n[Rendering]");
        println!("  Exposure: {:.2}x", self.exposure_multiplier);
        println!("  March Steps: {}", self.march_steps);
        println!("  Debug Mode: {}", self.debug_mode);
        println!(
            "  Channel λ: R={:.1}nm, G={:.1}nm, B={:.1}nm",
            self.uniform.channel_wavelengths[0],
            self.uniform.channel_wavelengths[1],
            self.uniform.channel_wavelengths[2]
        );
        println!("============================\n");
    }

    fn save_screenshot(&mut self) {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("output/screenshot_{}.png", timestamp);
        println!("Screenshot: {} (not implemented yet)", filename);
        // TODO: Implement screenshot
    }

    fn render(&mut self) -> Result<(), SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Viewer Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Viewer Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}

pub fn run(config_override: Option<&str>) -> Result<()> {
    env_logger::Builder::from_default_env().init();
    let args: Vec<String> = env::args().collect();
    let chosen = config_override
        .map(|s| s.to_string())
        .or_else(|| args.get(1).map(|s| s.to_string()))
        .unwrap_or_else(|| "configs/lut_config.toml".to_string());
    let config_path = chosen.as_str();

    let event_loop = EventLoop::new()?;
    let window = WindowBuilder::new()
        .with_title("Iron Rainbow 3D Viewer")
        .with_inner_size(PhysicalSize::new(1280, 720))
        .build(&event_loop)?;

    let mut state = pollster::block_on(ViewerState::new(window, config_path))?;

    event_loop.run(move |event, target| match event {
        Event::WindowEvent { window_id, event } if window_id == state.window().id() => {
            match event {
                WindowEvent::CloseRequested => {
                    target.exit();
                    return;
                }
                WindowEvent::Resized(size) => state.resize(size),
                WindowEvent::ScaleFactorChanged { .. } => {
                    state.resize(state.window().inner_size());
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key,
                            state: key_state,
                            ..
                        },
                    ..
                } => {
                    if key_state == ElementState::Pressed {
                        match logical_key {
                            Key::Character(ref ch) => {
                                let ch_lower = ch.to_lowercase();
                                match ch_lower.as_str() {
                                    "q" | "e" => {
                                        state.keys_pressed.insert(ch_lower.to_string());
                                    }
                                    "w" | "a" | "s" | "d" => {
                                        state.keys_pressed.insert(ch_lower.to_string());
                                    }
                                    "r" => {
                                        state
                                            .camera
                                            .reset(state.initial_camera_pos, state.initial_look_at);
                                        println!("Camera reset to default position");
                                    }
                                    "p" => state.print_status(),
                                    "0" => {
                                        state.uniform.debug_mode = 0;
                                        println!("Debug: Normal rendering");
                                    }
                                    "1" => {
                                        state.uniform.debug_mode = 1;
                                        println!("Debug: R channel only");
                                    }
                                    "2" => {
                                        state.uniform.debug_mode = 2;
                                        println!("Debug: G channel only");
                                    }
                                    "3" => {
                                        state.uniform.debug_mode = 3;
                                        println!("Debug: B channel only");
                                    }
                                    "4" => {
                                        state.uniform.debug_mode = 4;
                                        println!("Debug: LUT range check (green=in, red=out)");
                                    }
                                    "5" => {
                                        state.uniform.debug_mode = 5;
                                        println!("Debug: Anti-solar angle visualization");
                                    }
                                    "6" => {
                                        state.uniform.debug_mode = 6;
                                        println!("Debug: March distance visualization");
                                    }
                                    "7" => {
                                        state.uniform.debug_mode = 7;
                                        println!("Debug: Sphere hit test (red=hit, black=miss)");
                                    }
                                    "8" => {
                                        state.uniform.debug_mode = 8;
                                        println!("Debug: Ray direction");
                                    }
                                    "9" => {
                                        state.uniform.debug_mode = 9;
                                        println!("Debug: Test pattern (gradient)");
                                    }
                                    "=" | "+" => {
                                        state.exposure_multiplier *= 1.2;
                                        println!("Exposure: {:.2}x", state.exposure_multiplier);
                                    }
                                    "-" | "_" => {
                                        state.exposure_multiplier /= 1.2;
                                        println!("Exposure: {:.2}x", state.exposure_multiplier);
                                    }
                                    "[" => {
                                        state.march_steps = (state.march_steps / 2).max(8);
                                        println!("March steps: {}", state.march_steps);
                                    }
                                    "]" => {
                                        state.march_steps = (state.march_steps * 2).min(512);
                                        println!("March steps: {}", state.march_steps);
                                    }
                                    _ => {}
                                }
                            }
                            Key::Named(NamedKey::ArrowUp) => {
                                state.sun_elevation = (state.sun_elevation + 5.0).min(90.0);
                                println!("Sun elevation: {:.1}°", state.sun_elevation);
                            }
                            Key::Named(NamedKey::ArrowDown) => {
                                state.sun_elevation = (state.sun_elevation - 5.0).max(-90.0);
                                println!("Sun elevation: {:.1}°", state.sun_elevation);
                            }
                            Key::Named(NamedKey::ArrowLeft) => {
                                state.sun_azimuth = (state.sun_azimuth - 5.0 + 360.0) % 360.0;
                                println!("Sun azimuth: {:.1}°", state.sun_azimuth);
                            }
                            Key::Named(NamedKey::ArrowRight) => {
                                state.sun_azimuth = (state.sun_azimuth + 5.0) % 360.0;
                                println!("Sun azimuth: {:.1}°", state.sun_azimuth);
                            }
                            _ => {}
                        }
                    } else if key_state == ElementState::Released {
                        if let Key::Character(ref ch) = logical_key {
                            let ch_lower = ch.to_lowercase();
                            state.keys_pressed.remove(&ch_lower.to_string());
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } => {
                    // Capture mouse for camera look
                    let _ = state
                        .window
                        .set_cursor_grab(winit::window::CursorGrabMode::Locked);
                    state.window.set_cursor_visible(false);
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key: Key::Named(NamedKey::Escape),
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    // Release mouse
                    let _ = state
                        .window
                        .set_cursor_grab(winit::window::CursorGrabMode::None);
                    state.window.set_cursor_visible(true);
                }
                WindowEvent::RedrawRequested => {
                    state.update();
                    match state.render() {
                        Ok(()) => {}
                        Err(SurfaceError::Lost) => state.resize(state.size),
                        Err(SurfaceError::OutOfMemory) => target.exit(),
                        Err(e) => eprintln!("Render error: {:?}", e),
                    }
                }
                _ => {}
            }
        }
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => {
            state.handle_mouse_motion(delta.0, delta.1);
        }
        Event::AboutToWait => {
            state.window().request_redraw();
        }
        _ => {}
    })?;

    Ok(())
}

fn pack_false_color_data(metadata: &LutMetadata) -> ([[f32; 4]; MAX_FALSE_COLOR_STOPS], u32) {
    let mut packed = [[0.0_f32; 4]; MAX_FALSE_COLOR_STOPS];
    if metadata.false_color.is_empty() {
        return (packed, 0);
    }

    let mut stops = metadata.false_color.clone();
    stops.sort_by(|a, b| a.wavelength.partial_cmp(&b.wavelength).unwrap());

    let count = stops.len().min(MAX_FALSE_COLOR_STOPS);
    for (idx, stop) in stops.into_iter().take(MAX_FALSE_COLOR_STOPS).enumerate() {
        packed[idx][0] = stop.wavelength;
        packed[idx][1] = srgb_u8_to_linear(stop.color[0]);
        packed[idx][2] = srgb_u8_to_linear(stop.color[1]);
        packed[idx][3] = srgb_u8_to_linear(stop.color[2]);
    }

    (packed, count as u32)
}

fn srgb_u8_to_linear(value: u8) -> f32 {
    let v = value as f32 / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn resolve_wavelength_axis(metadata: &LutMetadata, config: &LutConfig) -> (f32, f32) {
    if let Some(axis) = normalize_axis(metadata.wavelength_min_nm, metadata.wavelength_max_nm) {
        return axis;
    }

    if let Some(axis) = normalize_axis(config.grid.wavelength_min_nm, config.grid.wavelength_max_nm)
    {
        warn!(
            "LUT metadata had invalid wavelength range; falling back to config ({:.1}–{:.1} nm)",
            config.grid.wavelength_min_nm, config.grid.wavelength_max_nm
        );
        return axis;
    }

    warn!("Unable to determine valid wavelength axis, defaulting to 230–300 nm");
    (230.0, 70.0)
}

fn resolve_angle_axis(metadata: &LutMetadata, config: &LutConfig) -> (f32, f32) {
    if let Some((min, range)) = normalize_axis(metadata.angle_min_deg, metadata.angle_max_deg) {
        if ranges_intersect(min, min + range, 0.0, 180.0) {
            return (min, range);
        } else {
            warn!(
                "Metadata angle window [{:.1}, {:.1}]° does not intersect physical scattering range; using config values",
                metadata.angle_min_deg, metadata.angle_max_deg
            );
        }
    }

    if let Some(axis) = normalize_axis(config.grid.angle_min_deg, config.grid.angle_max_deg) {
        return axis;
    }

    warn!("Unable to determine valid angle axis, defaulting to 0–180°");
    (0.0, 180.0)
}

fn normalize_axis(min: f32, max: f32) -> Option<(f32, f32)> {
    if !min.is_finite() || !max.is_finite() {
        return None;
    }
    let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
    let range = hi - lo;
    if range <= f32::EPSILON {
        None
    } else {
        Some((lo, range))
    }
}

fn ranges_intersect(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
    let (a_lo, a_hi) = if a0 <= a1 { (a0, a1) } else { (a1, a0) };
    let (b_lo, b_hi) = if b0 <= b1 { (b0, b1) } else { (b1, b0) };
    a_hi >= b_lo && b_hi >= a_lo
}

fn select_channel_wavelengths(metadata: &LutMetadata) -> [f32; 3] {
    if let Some(custom) = &metadata.channel_wavelengths {
        if custom.len() >= 3 {
            return [custom[0], custom[1], custom[2]];
        }
    }

    if metadata.false_color.len() >= 3 {
        let mut stops: Vec<f32> = metadata.false_color.iter().map(|s| s.wavelength).collect();
        stops.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let b = stops.first().cloned().unwrap_or(metadata.wavelength_min_nm);
        let g = stops[stops.len() / 2];
        let r = stops.last().cloned().unwrap_or(metadata.wavelength_max_nm);
        return [r, g, b];
    }

    let min = metadata.wavelength_min_nm;
    let max = metadata.wavelength_max_nm;
    [max, 0.5 * (min + max), min]
}

fn sun_direction(elevation_deg: f32, azimuth_deg: f32) -> [f32; 3] {
    // Spherical to Cartesian conversion
    // elevation: angle above XZ plane (like latitude)
    // azimuth: angle from +X axis in XZ plane (0° = +X, 90° = -Z, 180° = -X, 270° = +Z)
    // We rotate azimuth by 90° so that 0° points to +Z (forward)
    let elev = elevation_deg.to_radians();
    let azim = (azimuth_deg - 90.0).to_radians(); // Rotate by 90° so 0° = +Z

    let x = elev.cos() * azim.cos(); // cos(elev) * cos(azim-90)
    let y = elev.sin(); // sin(elev)
    let z = elev.cos() * azim.sin(); // cos(elev) * sin(azim-90)

    normalize([x, y, z])
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0001 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    normalize([
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ])
}

impl ParticleCloud {
    fn new(
        box_center: [f32; 3],
        box_size: [f32; 3],
        droplet_radius: f32,
        count: usize,
        seed: u64,
    ) -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        // Simple LCG random number generator (seeded)
        let mut rng_state = seed;
        let mut random = || -> f32 {
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            (rng_state as f32 / u64::MAX as f32)
        };

        let droplets: Vec<Droplet> = (0..count)
            .map(|_| {
                let x = box_center[0] + (random() - 0.5) * box_size[0];
                let y = box_center[1] + (random() - 0.5) * box_size[1];
                let z = box_center[2] + (random() - 0.5) * box_size[2];
                Droplet {
                    position: [x, y, z],
                    radius: droplet_radius,
                }
            })
            .collect();

        println!(
            "Particle Cloud: {} droplets in box [{:.1}, {:.1}, {:.1}] size [{:.1}×{:.1}×{:.1}]",
            droplets.len(),
            box_center[0],
            box_center[1],
            box_center[2],
            box_size[0],
            box_size[1],
            box_size[2]
        );

        Self {
            box_center,
            box_size,
            droplet_radius,
            droplets,
        }
    }
}

const VIEWER_WGSL: &str = r#"
struct ViewerUniform {
    camera_pos: vec3<f32>,
    _pad0: f32,
    camera_forward: vec3<f32>,
    _pad1: f32,
    camera_right: vec3<f32>,
    _pad2: f32,
    camera_up: vec3<f32>,
    _pad3: f32,
    sun_dir: vec3<f32>,
    _pad4: f32,
    droplet_center: vec3<f32>,
    droplet_radius: f32,
    viewport_width: f32,
    viewport_height: f32,
    fov: f32,
    exposure: f32,
    wavelength_min: f32,
    wavelength_range: f32,
    angle_min_deg: f32,
    angle_range_deg: f32,
    tex_width: u32,
    tex_height: u32,
    debug_mode: u32,
    march_steps: u32,
    channel_wavelengths: vec4<f32>,
};

@group(0) @binding(0) var lutTex: texture_2d<f32>;
@group(0) @binding(1) var lutSampler: sampler;
@group(0) @binding(2) var<uniform> params: ViewerUniform;

const PI: f32 = 3.14159265359;
const DEG_TO_RAD: f32 = 0.0174532925;
const RAD_TO_DEG: f32 = 57.2957795131;

// Ray-sphere intersection
fn intersect_sphere(ray_origin: vec3<f32>, ray_dir: vec3<f32>, sphere_center: vec3<f32>, sphere_radius: f32) -> vec2<f32> {
    let oc = ray_origin - sphere_center;
    let a = dot(ray_dir, ray_dir);
    let b = 2.0 * dot(oc, ray_dir);
    let c = dot(oc, oc) - sphere_radius * sphere_radius;
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return vec2<f32>(-1.0, -1.0);
    }

    let sqrt_d = sqrt(discriminant);
    let t0 = (-b - sqrt_d) / (2.0 * a);
    let t1 = (-b + sqrt_d) / (2.0 * a);

    return vec2<f32>(t0, t1);
}

// Sample LUT by anti-solar angle (degrees)
fn sample_lut_angle(angle_deg: f32, wavelength: f32) -> f32 {
    if (params.angle_range_deg <= 0.0 || params.wavelength_range <= 0.0) {
        return 0.0;
    }

    let tex_w = max(f32(params.tex_width), 1.0);
    let tex_h = max(f32(params.tex_height), 1.0);

    let angle_norm = clamp((angle_deg - params.angle_min_deg) / params.angle_range_deg, 0.0, 1.0);
    let wave_norm = clamp((wavelength - params.wavelength_min) / params.wavelength_range, 0.0, 1.0);

    let u = (angle_norm * (tex_w - 1.0) + 0.5) / tex_w;
    let v = (wave_norm * (tex_h - 1.0) + 0.5) / tex_h;

    // LUT coordinates: X=angle, Y=wavelength
    let uv = vec2<f32>(u, v);
    return textureSampleLevel(lutTex, lutSampler, uv, 0.0).r;
}

@vertex fn vs_main(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );
    let xy = positions[idx];
    return vec4<f32>(xy, 0.0, 1.0);
}

// Ray-plane intersection for grid
fn intersect_plane(ray_origin: vec3<f32>, ray_dir: vec3<f32>, plane_normal: vec3<f32>, plane_d: f32) -> f32 {
    let denom = dot(plane_normal, ray_dir);
    if (abs(denom) < 0.0001) {
        return -1.0;
    }
    let t = -(dot(plane_normal, ray_origin) + plane_d) / denom;
    return t;
}

// Draw grid on a plane
fn draw_grid(pos: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    let grid_size = 20.0;
    let line_width = 0.5;

    // Determine which axes to use based on normal
    var u: f32;
    var v: f32;
    if (abs(normal.y) > 0.9) {
        // XZ plane
        u = pos.x;
        v = pos.z;
    } else if (abs(normal.x) > 0.9) {
        // YZ plane
        u = pos.y;
        v = pos.z;
    } else {
        // XY plane
        u = pos.x;
        v = pos.y;
    }

    let grid_u = abs(fract(u / grid_size) - 0.5);
    let grid_v = abs(fract(v / grid_size) - 0.5);

    if (grid_u < line_width / grid_size || grid_v < line_width / grid_size) {
        return vec3<f32>(0.3, 0.3, 0.3); // Grid lines
    }

    // Axes
    if (abs(u) < line_width && abs(v) < 200.0) {
        return vec3<f32>(0.0, 0.8, 0.0); // Green axis
    }
    if (abs(v) < line_width && abs(u) < 200.0) {
        return vec3<f32>(0.0, 0.0, 0.8); // Blue axis
    }

    return vec3<f32>(0.0, 0.0, 0.0); // Transparent
}

@fragment fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Debug mode 9: Show test pattern
    if (params.debug_mode == 9u) {
        let uv = vec2<f32>(coord.x / params.viewport_width, coord.y / params.viewport_height);
        return vec4<f32>(uv.x, uv.y, 0.5, 1.0);
    }

    // Normalized device coordinates
    let ndc = vec2<f32>(
        (coord.x / params.viewport_width) * 2.0 - 1.0,
        1.0 - (coord.y / params.viewport_height) * 2.0
    );

    // Calculate aspect ratio
    let aspect = params.viewport_width / params.viewport_height;

    // Generate camera ray
    let fov_scale = tan(params.fov * 0.5);
    let ray_dir_cam = normalize(vec3<f32>(
        ndc.x * aspect * fov_scale,
        ndc.y * fov_scale,
        1.0
    ));

    // Transform to world space
    let ray_dir = normalize(
        ray_dir_cam.x * params.camera_right +
        ray_dir_cam.y * params.camera_up +
        ray_dir_cam.z * params.camera_forward
    );

    let ray_origin = params.camera_pos;

    // Debug mode 8: Show ray direction
    if (params.debug_mode == 8u) {
        return vec4<f32>(abs(ray_dir), 1.0);
    }

    // Draw reference grid (XZ plane at y=0)
    let grid_t = intersect_plane(ray_origin, ray_dir, vec3<f32>(0.0, 1.0, 0.0), 0.0);
    var background_color = vec3<f32>(0.0, 0.0, 0.05); // Dark blue

    if (grid_t > 0.0 && grid_t < 1000.0) {
        let grid_pos = ray_origin + ray_dir * grid_t;
        let grid_color = draw_grid(grid_pos, vec3<f32>(0.0, 1.0, 0.0));
        if (length(grid_color) > 0.01) {
            background_color = grid_color;
        }
    }

    // Draw sun direction indicator (yellow line from origin)
    let sun_line_t = intersect_sphere(
        vec3<f32>(0.0, 0.0, 0.0),
        params.sun_dir,
        params.sun_dir * 150.0,
        5.0
    );

    // Check if ray is close to sun direction line
    let to_sun = params.sun_dir * 150.0;
    let camera_to_sun = to_sun - ray_origin;
    let proj = dot(camera_to_sun, ray_dir);
    if (proj > 0.0) {
        let closest = ray_origin + ray_dir * proj;
        let dist_to_line = length(closest - (ray_origin + normalize(camera_to_sun) * proj));
        if (dist_to_line < 2.0 && proj < 300.0) {
            return vec4<f32>(1.0, 1.0, 0.0, 1.0); // Yellow sun indicator
        }
    }

    // Intersect with droplet sphere
    let hit = intersect_sphere(ray_origin, ray_dir, params.droplet_center, params.droplet_radius);

    // Debug mode 7: Show sphere hit (red = hit, black = miss)
    if (params.debug_mode == 7u) {
        if (hit.x < 0.0) {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        } else {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }
    }

    if (hit.x < 0.0) {
        // No intersection - show background with grid
        return vec4<f32>(background_color, 1.0);
    }

    // March through the droplet region
    let t_start = max(hit.x, 0.0);
    let t_end = hit.y;
    let march_dist = t_end - t_start;
    let step_size = march_dist / f32(params.march_steps);

    // Debug mode 6: Show march distance
    if (params.debug_mode == 6u) {
        let dist_norm = march_dist / 200.0;
        return vec4<f32>(dist_norm, dist_norm, dist_norm, 1.0);
    }

    var color = vec3<f32>(0.0);

    for (var i = 0u; i < params.march_steps; i++) {
        let t = t_start + (f32(i) + 0.5) * step_size;
        let pos = ray_origin + ray_dir * t;

        // Calculate anti-solar angle
        // dot(-ray_dir, sun_dir) gives scattering angle:
        //   1.0 = forward scattering (0°)
        //  -1.0 = backward scattering (180°)
        let cos_angle = dot(-ray_dir, params.sun_dir);
        let angle_rad = acos(clamp(cos_angle, -1.0, 1.0));
        let angle_deg_physical = angle_rad * RAD_TO_DEG; // 0° to 180°

        // LUT stores angles as -180° to -120°, which represents 120° to 180° backscattering
        // Map: 120° → -180°, 180° → -120°
        let angle_deg = -180.0 + (angle_deg_physical - 120.0);

        // Debug mode 5: Show anti-solar angle (physical, before mapping)
        if (params.debug_mode == 5u) {
            // Show physical angle (0-180) as grayscale
            let angle_norm = angle_deg_physical / 180.0;
            return vec4<f32>(angle_norm, 0.0, 0.0, 1.0); // Red channel = angle
        }

        // Debug mode 4: Show LUT range check
        if (params.debug_mode == 4u) {
            let angle_norm = (angle_deg - params.angle_min_deg) / params.angle_range_deg;
            if (angle_norm >= 0.0 && angle_norm <= 1.0) {
                return vec4<f32>(0.0, 1.0, 0.0, 1.0); // Green = in range
            } else {
                return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red = out of range
            }
        }

        // Sample each channel from LUT
        let channels = params.channel_wavelengths;
        let intensityR = sample_lut_angle(angle_deg, channels.x);
        let intensityG = sample_lut_angle(angle_deg, channels.y);
        let intensityB = sample_lut_angle(angle_deg, channels.z);

        // Accumulate color
        if (params.debug_mode == 1u) {
            color += vec3<f32>(intensityR, 0.0, 0.0);
        } else if (params.debug_mode == 2u) {
            color += vec3<f32>(0.0, intensityG, 0.0);
        } else if (params.debug_mode == 3u) {
            color += vec3<f32>(0.0, 0.0, intensityB);
        } else {
            color += vec3<f32>(intensityR, intensityG, intensityB);
        }
    }

    // Normalize by number of steps
    color /= f32(params.march_steps);

    // Apply exposure and boost for debugging
    color *= params.exposure * 100.0; // Temporary boost to see if there's any signal

    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}
"#;
