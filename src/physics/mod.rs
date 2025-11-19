pub mod absorption;
pub mod dispersion;
/// Physics simulation algorithms
/// Ray tracing, refraction, absorption, and material properties
pub mod intersection;
pub mod path_trace;
pub mod refraction;

pub use absorption::{absorbance, beer_lambert, path_length_2d, transmittance};
pub use dispersion::{
    wavelengths, AbsorptionCoefficient, CauchyModel, DispersionModel, DrudeModel, RefractiveIndex,
    SellmeierModel, Wavelength,
};
pub use intersection::{compute_intersections, IntersectionResult};
pub use path_trace::{compute_path_traces, EventType, PathTraceInput, PathTraceResult};
pub use refraction::{compute_refractions, RefractionInput, RefractionResult};
