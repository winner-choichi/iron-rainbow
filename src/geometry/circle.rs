/// Circle geometry for ray tracing
/// Represents a 2D circle (cross-section of spherical particle)

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Circle {
    pub center: [f32; 2],
    pub radius: f32,
    pub _padding: f32,  // Align to 16 bytes for GPU
}

impl Circle {
    /// Create a new circle
    pub fn new(center: [f32; 2], radius: f32) -> Self {
        Self {
            center,
            radius,
            _padding: 0.0,
        }
    }

    /// Check if point is inside circle
    pub fn contains(&self, point: [f32; 2]) -> bool {
        let dx = point[0] - self.center[0];
        let dy = point[1] - self.center[1];
        dx * dx + dy * dy <= self.radius * self.radius
    }

    /// Get normal vector at point on circle surface (pointing outward)
    pub fn normal_at(&self, point: [f32; 2]) -> [f32; 2] {
        let dx = point[0] - self.center[0];
        let dy = point[1] - self.center[1];
        let len = (dx * dx + dy * dy).sqrt();
        [dx / len, dy / len]
    }
}
