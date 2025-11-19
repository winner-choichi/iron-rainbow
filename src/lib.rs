pub mod geometry;
/// Iron Rainbow: Liquid Steel Particle Rainbow Simulation
/// A high-performance GPU-accelerated physics simulation
///
/// Phase 1: 2D Ray Tracing Logic Verification
pub mod gpu;
pub mod lut;
pub mod physics;
pub mod shaders;
pub mod viewer_app;
pub mod visualization;

// Re-export commonly used types
pub use geometry::{Circle, Ray};
pub use gpu::GpuContext;
pub use lut::{
    DropletConfig, FalseColorStop, GridConfig, IntensityConfig, LutConfig, OutputConfig,
    RendererConfig, WavelengthBatchConfig,
};
pub use physics::{
    absorbance, beer_lambert, compute_intersections, compute_path_traces, compute_refractions,
    path_length_2d, transmittance, wavelengths, AbsorptionCoefficient, CauchyModel,
    DispersionModel, DrudeModel, EventType, IntersectionResult, PathTraceInput, PathTraceResult,
    RefractionInput, RefractionResult, RefractiveIndex, SellmeierModel, Wavelength,
};
pub use visualization::Renderer2D;
