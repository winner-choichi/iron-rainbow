/// Ray geometry for ray tracing
/// Represents a 2D ray with origin and direction

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ray {
    pub origin: [f32; 2],
    pub direction: [f32; 2],
}

impl Ray {
    /// Create a new ray
    pub fn new(origin: [f32; 2], direction: [f32; 2]) -> Self {
        Self { origin, direction }
    }

    /// Create a normalized ray (direction is unit vector)
    pub fn normalized(origin: [f32; 2], direction: [f32; 2]) -> Self {
        let len = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
        Self {
            origin,
            direction: [direction[0] / len, direction[1] / len],
        }
    }

    /// Get point along ray at distance t
    pub fn at(&self, t: f32) -> [f32; 2] {
        [
            self.origin[0] + t * self.direction[0],
            self.origin[1] + t * self.direction[1],
        ]
    }
}
