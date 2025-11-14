/// Ray path tracing through a circular particle
/// Tracks entry, internal reflection, and exit events

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::{GpuContext, Ray, Circle};

/// Input for path tracing computation
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PathTraceInput {
    pub ray_origin: [f32; 2],
    pub ray_direction: [f32; 2],
    pub circle_center: [f32; 2],
    pub circle_radius: f32,
    pub n_outside: f32,
    pub n_inside: f32,
    pub padding1: f32,
    pub padding2: f32,
    pub padding3: f32,
}

impl PathTraceInput {
    pub fn new(ray: &Ray, circle: &Circle, n_outside: f32, n_inside: f32) -> Self {
        Self {
            ray_origin: ray.origin,
            ray_direction: ray.direction,
            circle_center: circle.center,
            circle_radius: circle.radius,
            n_outside,
            n_inside,
            padding1: 0.0,
            padding2: 0.0,
            padding3: 0.0,
        }
    }
}

/// Event type in the ray path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    None = 0,
    Refraction = 1,
    Reflection = 2,
    TotalInternalReflection = 3,
}

impl From<u32> for EventType {
    fn from(value: u32) -> Self {
        match value {
            1 => EventType::Refraction,
            2 => EventType::Reflection,
            3 => EventType::TotalInternalReflection,
            _ => EventType::None,
        }
    }
}

/// Result of path tracing computation
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PathTraceResult {
    // Event 0: Entry (refraction into particle)
    pub event0_point: [f32; 2],
    pub event0_direction: [f32; 2],
    pub event0_type: u32,
    pub _padding0: u32,  // Align next vec2

    // Event 1: Internal reflection
    pub event1_point: [f32; 2],
    pub event1_direction: [f32; 2],
    pub event1_type: u32,
    pub _padding1: u32,  // Align next vec2

    // Event 2: Exit (refraction out of particle)
    pub event2_point: [f32; 2],
    pub event2_direction: [f32; 2],
    pub event2_type: u32,
    pub _padding2: u32,  // Align next field

    pub num_events: u32,
    pub _padding3: u32,
}

impl PathTraceResult {
    pub fn event0_type(&self) -> EventType {
        EventType::from(self.event0_type)
    }

    pub fn event1_type(&self) -> EventType {
        EventType::from(self.event1_type)
    }

    pub fn event2_type(&self) -> EventType {
        EventType::from(self.event2_type)
    }

    pub fn has_event0(&self) -> bool {
        self.num_events >= 1
    }

    pub fn has_event1(&self) -> bool {
        self.num_events >= 2
    }

    pub fn has_event2(&self) -> bool {
        self.num_events >= 3
    }
}

/// Compute path traces for multiple rays on GPU
pub async fn compute_path_traces(
    gpu: &GpuContext,
    inputs: &[PathTraceInput],
) -> Vec<PathTraceResult> {
    let shader_code = include_str!("../shaders/path_trace.wgsl");

    let shader_module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Path Trace Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_code.into()),
    });

    // Create input buffer
    let input_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Path Trace Input Buffer"),
        contents: bytemuck::cast_slice(inputs),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    // Create output buffer
    let output_size = (inputs.len() * std::mem::size_of::<PathTraceResult>()) as u64;
    let output_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Path Trace Output Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    // Create staging buffer
    let staging_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Path Trace Staging Buffer"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create bind group layout
    let bind_group_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Path Trace Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    // Create bind group
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Path Trace Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    // Create pipeline layout
    let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Path Trace Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create compute pipeline
    let compute_pipeline = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Path Trace Pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: "main",
        compilation_options: Default::default(),
        cache: None,
    });

    // Create command encoder and dispatch compute shader
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Path Trace Command Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Path Trace Compute Pass"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        let workgroup_count = ((inputs.len() as f32) / 64.0).ceil() as u32;
        compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    }

    // Copy output to staging buffer
    encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging_buffer, 0, output_size);

    gpu.queue.submit(Some(encoder.finish()));

    // Read back results
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).unwrap();
    });

    gpu.device.poll(wgpu::Maintain::Wait);
    receiver.receive().await.unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let results: Vec<PathTraceResult> = bytemuck::cast_slice(&data).to_vec();

    drop(data);
    staging_buffer.unmap();

    results
}
