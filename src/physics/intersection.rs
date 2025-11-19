/// Ray-circle intersection computation on GPU
/// High-performance parallel intersection testing
use crate::geometry::{Circle, Ray};
use crate::gpu::{execute_and_read, BufferManager, ComputePipeline, GpuContext};
use crate::shaders;

/// Intersection result from GPU computation
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct IntersectionResult {
    pub hit: f32,     // 0.0 = miss, 1.0 = hit
    pub t: f32,       // distance along ray
    pub point_x: f32, // intersection point x
    pub point_y: f32, // intersection point y
}

impl IntersectionResult {
    /// Check if intersection occurred
    pub fn is_hit(&self) -> bool {
        self.hit > 0.5
    }

    /// Get intersection point as array
    pub fn point(&self) -> [f32; 2] {
        [self.point_x, self.point_y]
    }
}

/// Compute ray-circle intersections on GPU
pub async fn compute_intersections(
    gpu: &GpuContext,
    rays: &[Ray],
    circle: &Circle,
) -> Vec<IntersectionResult> {
    let device = &gpu.device;
    let queue = &gpu.queue;

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Intersection Bind Group Layout"),
        entries: &[
            // Rays buffer (read-only)
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
            // Circle buffer (read-only)
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Results buffer (read-write)
            wgpu::BindGroupLayoutEntry {
                binding: 2,
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

    // Create compute pipeline
    let pipeline = ComputePipeline::new(
        device,
        shaders::INTERSECTION_SHADER,
        "main",
        bind_group_layout,
        Some("Ray-Circle Intersection Pipeline"),
    );

    // Create GPU buffers
    let rays_buffer = BufferManager::create_storage_buffer_init(device, "Rays Buffer", rays);
    let circle_buffer =
        BufferManager::create_storage_buffer_init(device, "Circle Buffer", &[*circle]);

    let results_size = (rays.len() * std::mem::size_of::<IntersectionResult>()) as u64;
    let results_buffer =
        BufferManager::create_storage_buffer(device, "Results Buffer", results_size);

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Intersection Bind Group"),
        layout: &pipeline.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: rays_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: circle_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: results_buffer.as_entire_binding(),
            },
        ],
    });

    // Execute on GPU and read results
    let workgroups = ((rays.len() as u32 + 63) / 64, 1, 1); // 64 threads per workgroup
    execute_and_read(
        device,
        queue,
        &pipeline.pipeline,
        &bind_group,
        &results_buffer,
        workgroups,
        rays.len(),
    )
    .await
}
