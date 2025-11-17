/// Iron Rainbow: Liquid Steel Particle Rainbow Simulation
/// A high-performance GPU-accelerated physics simulation
///
/// Phase 1: 2D Ray Tracing Logic Verification

pub mod gpu;
pub mod shaders;
pub mod geometry;
pub mod physics;
pub mod visualization;
pub mod lut;
pub mod viewer_app;

// Re-export commonly used types
pub use gpu::GpuContext;
pub use geometry::{Ray, Circle};
pub use physics::{
    IntersectionResult, compute_intersections,
    RefractionInput, RefractionResult, compute_refractions,
    PathTraceInput, PathTraceResult, EventType, compute_path_traces,
    DispersionModel, CauchyModel, SellmeierModel, DrudeModel,
    Wavelength, RefractiveIndex, AbsorptionCoefficient, wavelengths,
    beer_lambert, transmittance, absorbance, path_length_2d,
};
pub use visualization::Renderer2D;
pub use lut::{
    LutConfig,
    GridConfig,
    IntensityConfig,
    DropletConfig,
    WavelengthBatchConfig,
    OutputConfig,
    RendererConfig,
    FalseColorStop,
};
