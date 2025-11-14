/// Compute pipeline management for GPU shaders

use wgpu::util::DeviceExt;

pub struct ComputePipeline {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl ComputePipeline {
    /// Create a compute pipeline from WGSL shader source
    pub fn new(
        device: &wgpu::Device,
        shader_source: &str,
        entry_point: &str,
        bind_group_layout: wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point,
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            pipeline,
            bind_group_layout,
        }
    }
}

/// Helper for creating GPU buffers
pub struct BufferManager;

impl BufferManager {
    /// Create a storage buffer initialized with data
    pub fn create_storage_buffer_init<T: bytemuck::Pod>(
        device: &wgpu::Device,
        label: &str,
        data: &[T],
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE,
        })
    }

    /// Create a storage buffer for output (read-write)
    pub fn create_storage_buffer(
        device: &wgpu::Device,
        label: &str,
        size: u64,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        })
    }

    /// Create a staging buffer for reading GPU results back to CPU
    pub fn create_staging_buffer(
        device: &wgpu::Device,
        label: &str,
        size: u64,
    ) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
}

/// Execute compute shader and read results back to CPU
pub async fn execute_and_read<T: bytemuck::Pod + Copy>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    output_buffer: &wgpu::Buffer,
    workgroups: (u32, u32, u32),
    result_count: usize,
) -> Vec<T> {
    // Create staging buffer
    let buffer_size = (result_count * std::mem::size_of::<T>()) as u64;
    let staging_buffer = BufferManager::create_staging_buffer(device, "Staging Buffer", buffer_size);

    // Execute compute shader
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Compute Encoder"),
    });

    {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Compute Pass"),
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(pipeline);
        compute_pass.set_bind_group(0, bind_group, &[]);
        compute_pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
    }

    // Copy results to staging buffer
    encoder.copy_buffer_to_buffer(output_buffer, 0, &staging_buffer, 0, buffer_size);
    queue.submit(Some(encoder.finish()));

    // Read results back to CPU
    let buffer_slice = staging_buffer.slice(..);
    let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

    device.poll(wgpu::Maintain::Wait);

    if let Some(Ok(())) = receiver.receive().await {
        let data = buffer_slice.get_mapped_range();
        let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();
        result
    } else {
        panic!("Failed to read GPU buffer");
    }
}
