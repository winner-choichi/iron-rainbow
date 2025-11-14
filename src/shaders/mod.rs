/// WGSL shader sources for GPU compute kernels

/// Ray-circle intersection shader
pub const INTERSECTION_SHADER: &str = include_str!("intersection.wgsl");

/// Snell's law refraction shader
pub const REFRACTION_SHADER: &str = include_str!("refraction.wgsl");
