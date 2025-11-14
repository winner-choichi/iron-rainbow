/// Physics simulation algorithms
/// Ray tracing, refraction, absorption, and material properties

pub mod intersection;
pub mod refraction;
pub mod path_trace;
pub mod dispersion;
pub mod absorption;

pub use intersection::{IntersectionResult, compute_intersections};
pub use refraction::{RefractionInput, RefractionResult, compute_refractions};
pub use path_trace::{PathTraceInput, PathTraceResult, EventType, compute_path_traces};
pub use dispersion::{
    DispersionModel, CauchyModel, SellmeierModel, DrudeModel,
    Wavelength, RefractiveIndex, AbsorptionCoefficient, wavelengths,
};
pub use absorption::{beer_lambert, transmittance, absorbance, path_length_2d};
