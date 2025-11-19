/// Beer-Lambert absorption law
/// Calculates intensity attenuation through absorbing media
use crate::physics::AbsorptionCoefficient;

/// Calculate transmitted intensity using Beer-Lambert law
/// I = I₀ × exp(-α × d)
///
/// # Arguments
/// * `incident_intensity` - Initial intensity I₀
/// * `absorption_coeff` - Absorption coefficient α (in μm⁻¹)
/// * `path_length` - Path length d through material (in μm)
///
/// # Returns
/// Transmitted intensity I
pub fn beer_lambert(
    incident_intensity: f32,
    absorption_coeff: AbsorptionCoefficient,
    path_length: f32,
) -> f32 {
    incident_intensity * (-absorption_coeff * path_length).exp()
}

/// Calculate transmittance (ratio of transmitted to incident intensity)
pub fn transmittance(absorption_coeff: AbsorptionCoefficient, path_length: f32) -> f32 {
    (-absorption_coeff * path_length).exp()
}

/// Calculate absorbance (optical density)
/// A = -log₁₀(I/I₀) = α × d / ln(10)
pub fn absorbance(absorption_coeff: AbsorptionCoefficient, path_length: f32) -> f32 {
    absorption_coeff * path_length / 10_f32.ln()
}

/// Calculate path length between two 2D points
pub fn path_length_2d(p1: [f32; 2], p2: [f32; 2]) -> f32 {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beer_lambert_no_absorption() {
        let intensity = beer_lambert(1.0, 0.0, 100.0);
        assert!(
            (intensity - 1.0).abs() < 1e-6,
            "No absorption should preserve intensity"
        );
    }

    #[test]
    fn test_beer_lambert_full_absorption() {
        let intensity = beer_lambert(1.0, 100.0, 100.0);
        assert!(intensity < 1e-6, "High absorption should block light");
    }

    #[test]
    fn test_transmittance_half() {
        // Find path length that gives 50% transmittance
        let alpha = 0.693; // ln(2)
        let path = 1.0;
        let t = transmittance(alpha, path);
        assert!((t - 0.5).abs() < 0.01, "Should give ~50% transmittance");
    }

    #[test]
    fn test_path_length() {
        let p1 = [0.0, 0.0];
        let p2 = [3.0, 4.0];
        let length = path_length_2d(p1, p2);
        assert!((length - 5.0).abs() < 1e-6, "3-4-5 triangle");
    }
}
