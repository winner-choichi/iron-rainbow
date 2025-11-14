/// Snell's Law refraction computation on GPU
/// Handles refraction and total internal reflection

use crate::geometry::Ray;
use crate::gpu::{GpuContext, ComputePipeline, BufferManager, execute_and_read};
use crate::shaders;

/// Input data for refraction calculation
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RefractionInput {
    pub ray_origin: [f32; 2],
    pub ray_direction: [f32; 2],
    pub intersection_point: [f32; 2],
    pub normal: [f32; 2],
    pub n1: f32,  // Refractive index of incident medium
    pub n2: f32,  // Refractive index of refracted medium
    pub _padding1: f32,
    pub _padding2: f32,
}

impl RefractionInput {
    /// Create refraction input from ray, intersection point, normal, and refractive indices
    pub fn new(
        ray: &Ray,
        intersection_point: [f32; 2],
        normal: [f32; 2],
        n1: f32,
        n2: f32,
    ) -> Self {
        Self {
            ray_origin: ray.origin,
            ray_direction: ray.direction,
            intersection_point,
            normal,
            n1,
            n2,
            _padding1: 0.0,
            _padding2: 0.0,
        }
    }
}

/// Refraction result from GPU computation
#[repr(C, align(8))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RefractionResult {
    pub refracted_direction_x: f32,
    pub refracted_direction_y: f32,
    pub total_internal_reflection: f32,  // 1.0 = TIR, 0.0 = normal refraction
    pub reflected_direction_x: f32,
    pub reflected_direction_y: f32,
    pub _padding1: f32,
    pub _padding2: f32,
    pub _padding3: f32,
}

impl RefractionResult {
    /// Get refracted direction as array
    pub fn refracted_direction(&self) -> [f32; 2] {
        [self.refracted_direction_x, self.refracted_direction_y]
    }

    /// Get reflected direction as array
    pub fn reflected_direction(&self) -> [f32; 2] {
        [self.reflected_direction_x, self.reflected_direction_y]
    }
}

impl RefractionResult {
    /// Check if total internal reflection occurred
    pub fn is_tir(&self) -> bool {
        self.total_internal_reflection > 0.5
    }

    /// Get refracted ray (only valid if not TIR)
    pub fn get_refracted_ray(&self, origin: [f32; 2]) -> Ray {
        Ray::new(origin, self.refracted_direction())
    }

    /// Get reflected ray
    pub fn get_reflected_ray(&self, origin: [f32; 2]) -> Ray {
        Ray::new(origin, self.reflected_direction())
    }

    /// Get angle of refraction in degrees (from normal)
    pub fn refraction_angle_degrees(&self) -> f32 {
        if self.is_tir() {
            return 0.0;
        }
        let dir = self.refracted_direction();
        let cos_theta = (dir[0] * dir[0] + dir[1] * dir[1]).sqrt();
        cos_theta.acos().to_degrees()
    }
}

/// Compute refractions on GPU
pub async fn compute_refractions(
    gpu: &GpuContext,
    inputs: &[RefractionInput],
) -> Vec<RefractionResult> {
    let device = &gpu.device;
    let queue = &gpu.queue;

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Refraction Bind Group Layout"),
        entries: &[
            // Input buffer (read-only)
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
            // Results buffer (read-write)
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

    // Create compute pipeline
    let pipeline = ComputePipeline::new(
        device,
        shaders::REFRACTION_SHADER,
        "main",
        bind_group_layout,
        Some("Refraction Pipeline"),
    );

    // Create GPU buffers
    let inputs_buffer = BufferManager::create_storage_buffer_init(device, "Inputs Buffer", inputs);

    let results_size = (inputs.len() * std::mem::size_of::<RefractionResult>()) as u64;
    let results_buffer = BufferManager::create_storage_buffer(device, "Results Buffer", results_size);

    // Create bind group
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Refraction Bind Group"),
        layout: &pipeline.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: inputs_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: results_buffer.as_entire_binding(),
            },
        ],
    });

    // Execute on GPU and read results
    let workgroups = ((inputs.len() as u32 + 63) / 64, 1, 1);
    execute_and_read(
        device,
        queue,
        &pipeline.pipeline,
        &bind_group,
        &results_buffer,
        workgroups,
        inputs.len(),
    ).await
}
