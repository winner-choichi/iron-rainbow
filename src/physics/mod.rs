/// Physics simulation algorithms
/// Ray tracing, refraction, absorption, and material properties

pub mod intersection;
pub mod refraction;

pub use intersection::{IntersectionResult, compute_intersections};
pub use refraction::{RefractionInput, RefractionResult, compute_refractions};
