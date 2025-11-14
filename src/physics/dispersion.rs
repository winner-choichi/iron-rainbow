/// Wavelength-dependent refractive index
/// Implements dispersion models for materials

/// Wavelength in nanometers
pub type Wavelength = f32;

/// Refractive index (real part)
pub type RefractiveIndex = f32;

/// Absorption coefficient (imaginary part)
pub type AbsorptionCoefficient = f32;

/// Material dispersion model
pub trait DispersionModel {
    /// Get refractive index at given wavelength (nm)
    fn refractive_index(&self, wavelength: Wavelength) -> RefractiveIndex;

    /// Get absorption coefficient at given wavelength (nm)
    fn absorption_coefficient(&self, wavelength: Wavelength) -> AbsorptionCoefficient;
}

/// Cauchy's equation for transparent materials (glass, water, etc.)
/// n(λ) = A + B/λ² + C/λ⁴
/// Valid for visible spectrum, no absorption
pub struct CauchyModel {
    pub a: f32,  // Constant term
    pub b: f32,  // λ⁻² coefficient (in μm²)
    pub c: f32,  // λ⁻⁴ coefficient (in μm⁴)
}

impl CauchyModel {
    /// Standard optical glass (BK7-like)
    pub fn glass() -> Self {
        Self {
            a: 1.458,
            b: 0.00354,  // μm²
            c: 0.0,
        }
    }

    /// Water
    pub fn water() -> Self {
        Self {
            a: 1.3247,
            b: 0.00307,
            c: 0.0,
        }
    }
}

impl DispersionModel for CauchyModel {
    fn refractive_index(&self, wavelength_nm: Wavelength) -> RefractiveIndex {
        let lambda_um = wavelength_nm / 1000.0;  // Convert nm to μm
        let lambda2 = lambda_um * lambda_um;
        let lambda4 = lambda2 * lambda2;

        self.a + self.b / lambda2 + self.c / lambda4
    }

    fn absorption_coefficient(&self, _wavelength_nm: Wavelength) -> AbsorptionCoefficient {
        0.0  // Transparent materials have no absorption
    }
}

/// Sellmeier equation for more accurate dispersion
/// n²(λ) - 1 = Σ(Bᵢλ²)/(λ² - Cᵢ)
pub struct SellmeierModel {
    pub b1: f32,
    pub b2: f32,
    pub b3: f32,
    pub c1: f32,  // in μm²
    pub c2: f32,
    pub c3: f32,
}

impl SellmeierModel {
    /// BK7 glass (common optical glass)
    pub fn bk7() -> Self {
        Self {
            b1: 1.03961212,
            b2: 0.231792344,
            b3: 1.01046945,
            c1: 0.00600069867,  // μm²
            c2: 0.0200179144,
            c3: 103.560653,
        }
    }
}

impl DispersionModel for SellmeierModel {
    fn refractive_index(&self, wavelength_nm: Wavelength) -> RefractiveIndex {
        let lambda_um = wavelength_nm / 1000.0;
        let lambda2 = lambda_um * lambda_um;

        let term1 = (self.b1 * lambda2) / (lambda2 - self.c1);
        let term2 = (self.b2 * lambda2) / (lambda2 - self.c2);
        let term3 = (self.b3 * lambda2) / (lambda2 - self.c3);

        let n_squared = 1.0 + term1 + term2 + term3;
        n_squared.sqrt()
    }

    fn absorption_coefficient(&self, _wavelength_nm: Wavelength) -> AbsorptionCoefficient {
        0.0
    }
}

/// Drude model for metals (simplified)
/// For future implementation of steel dispersion
pub struct DrudeModel {
    pub plasma_frequency: f32,  // ωₚ (rad/s)
    pub damping: f32,           // γ (rad/s)
}

impl DrudeModel {
    /// Placeholder for steel (to be refined with real data)
    pub fn steel_placeholder() -> Self {
        Self {
            plasma_frequency: 1.0e16,  // ~UV region
            damping: 1.0e14,
        }
    }
}

impl DispersionModel for DrudeModel {
    fn refractive_index(&self, _wavelength_nm: Wavelength) -> RefractiveIndex {
        // Simplified - needs proper complex calculation
        // For now, return a constant
        1.5
    }

    fn absorption_coefficient(&self, _wavelength_nm: Wavelength) -> AbsorptionCoefficient {
        // Metals have high absorption in visible
        1.0
    }
}

/// Visible spectrum wavelengths
pub mod wavelengths {
    use super::Wavelength;

    pub const VIOLET: Wavelength = 400.0;  // nm
    pub const BLUE: Wavelength = 450.0;
    pub const CYAN: Wavelength = 500.0;
    pub const GREEN: Wavelength = 550.0;
    pub const YELLOW: Wavelength = 580.0;
    pub const ORANGE: Wavelength = 600.0;
    pub const RED: Wavelength = 700.0;

    /// Sample visible spectrum with given number of points
    pub fn sample_visible(num_samples: usize) -> Vec<Wavelength> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / (num_samples - 1) as f32;
                VIOLET + t * (RED - VIOLET)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cauchy_glass() {
        let glass = CauchyModel::glass();

        // Blue light should have higher refractive index than red
        let n_blue = glass.refractive_index(450.0);
        let n_red = glass.refractive_index(700.0);

        assert!(n_blue > n_red, "Blue should refract more than red");
        assert!(n_blue > 1.4 && n_blue < 1.6);
        assert!(n_red > 1.4 && n_red < 1.6);
    }

    #[test]
    fn test_sellmeier_bk7() {
        let bk7 = SellmeierModel::bk7();

        let n_blue = bk7.refractive_index(450.0);
        let n_red = bk7.refractive_index(700.0);

        assert!(n_blue > n_red);
    }
}
