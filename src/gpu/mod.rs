/// GPU computation infrastructure
/// Manages wgpu context, pipelines, and buffer operations
pub mod context;
pub mod pipeline;

pub use context::GpuContext;
pub use pipeline::{execute_and_read, BufferManager, ComputePipeline};
