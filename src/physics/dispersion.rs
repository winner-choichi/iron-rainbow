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

/// Lorentz oscillator for interband transitions
#[derive(Clone, Copy, Debug)]
pub struct LorentzOscillator {
    pub strength: f32,      // fⱼ (oscillator strength)
    pub frequency: f32,     // ωⱼ (resonance frequency, rad/s)
    pub width: f32,         // Γⱼ (damping width, rad/s)
}

/// Drude-Lorentz model for metals
/// Calculates complex refractive index: n + ik
///
/// Full Drude-Lorentz model:
/// ε(ω) = ε∞ - ωₚ²/(ω² + iγω) + Σⱼ [fⱼωₚ²/(ωⱼ² - ω² - iΓⱼω)]
/// n + ik = √ε(ω)
///
/// Components:
/// - ε∞: high-frequency dielectric constant (bound electrons)
/// - Drude term: free electron plasma response
/// - Lorentz terms: interband transitions (UV absorption)
pub struct DrudeModel {
    pub epsilon_inf: f32,       // ε∞ (high-frequency dielectric constant)
    pub plasma_frequency: f32,  // ωₚ (rad/s)
    pub damping: f32,           // γ (rad/s, Drude damping)
    pub oscillators: Vec<LorentzOscillator>,  // Interband transitions
}

impl DrudeModel {
    /// Steel/Iron parameters (Full Drude-Lorentz model)
    /// Based on Johnson & Christy (1974) experimental data
    /// Fitted to match UV-visible optical constants (188-400nm)
    pub fn steel() -> Self {
        // Lorentz oscillators for interband transitions
        // Fitted to reproduce Fe optical constants around 180-250nm
        let oscillators = vec![
            // Far-UV bound electrons (λ ≈ 80 nm) boost ε₁ so that n > 1 below plasma λ
            LorentzOscillator {
                strength: 1.2,
                frequency: 2.4e16,          // ω ≈ 78 nm
                width: 5.0e15,
            },
            // Near-UV interband transition (λ ≈ 160 nm)
            LorentzOscillator {
                strength: 0.8,
                frequency: 1.2e16,
                width: 3.5e15,
            },
            // Visible/UV tail (λ ≈ 220-260 nm)
            LorentzOscillator {
                strength: 0.35,
                frequency: 8.5e15,
                width: 2.0e15,
            },
        ];

        Self {
            epsilon_inf: 2.4,           // ε∞ tuned to match Johnson & Christy UV data
            plasma_frequency: 1.37e16,  // ~1.37 × 10^16 rad/s (UV region, ~137 nm)
            damping: 4.0e13,            // ~4 × 10^13 rad/s (Drude damping)
            oscillators,
        }
    }

    /// Calculate complex refractive index at given wavelength
    /// Returns (n, k) where n + ik is the complex index
    pub fn complex_index(&self, wavelength_nm: Wavelength) -> (f32, f32) {
        // Convert wavelength to angular frequency
        // ω = 2πc/λ
        let c = 2.998e17;  // Speed of light in nm/s
        let omega = 2.0 * std::f32::consts::PI * c / wavelength_nm;

        let omega_p = self.plasma_frequency;
        let gamma = self.damping;

        // Normalize frequencies to avoid overflow
        // Let x = ω/ωₚ and g = γ/ωₚ
        let x = omega / omega_p;
        let g = gamma / omega_p;

        // Drude term: -1/(x² + igx)
        let x2 = x * x;
        let g2 = g * g;
        let denom_drude = x2 + g2;

        let eps1_drude = -1.0 / denom_drude;
        let eps2_drude = g / (x * denom_drude);

        // Lorentz terms: Σⱼ [fⱼ/(ωⱼ²/ωₚ² - x² - i·Γⱼ/ωₚ·x)]
        let mut eps1_lorentz = 0.0;
        let mut eps2_lorentz = 0.0;

        for osc in &self.oscillators {
            let xj = osc.frequency / omega_p;      // ωⱼ/ωₚ
            let gj = osc.width / omega_p;          // Γⱼ/ωₚ
            let fj = osc.strength;

            let xj2 = xj * xj;
            let delta = xj2 - x2;                   // ωⱼ² - ω²

            // Denominator: (ωⱼ² - ω²)² + (Γⱼω)²
            let denom_lor = delta * delta + (gj * x) * (gj * x);

            // fⱼ(ωⱼ² - ω² + iΓⱼω) / denom
            eps1_lorentz += fj * delta / denom_lor;
            eps2_lorentz += fj * gj * x / denom_lor;
        }

        // Total permittivity: ε(ω) = ε∞ + Drude + Lorentz
        let eps1 = self.epsilon_inf + eps1_drude + eps1_lorentz;
        let eps2 = eps2_drude + eps2_lorentz;

        // Complex square root: √(ε₁ + iε₂)
        // n = √((|ε| + ε₁)/2)
        // k = √((|ε| - ε₁)/2)
        let eps_mag = (eps1 * eps1 + eps2 * eps2).sqrt();

        let n = ((eps_mag + eps1) / 2.0).max(0.0).sqrt();
        let k = ((eps_mag - eps1) / 2.0).max(0.0).sqrt();

        (n, k)
    }
}

impl DispersionModel for DrudeModel {
    fn refractive_index(&self, wavelength_nm: Wavelength) -> RefractiveIndex {
        let (n, _k) = self.complex_index(wavelength_nm);
        n
    }

    fn absorption_coefficient(&self, wavelength_nm: Wavelength) -> AbsorptionCoefficient {
        let (_n, k) = self.complex_index(wavelength_nm);
        // Convert extinction coefficient k to absorption coefficient α
        // α = 4πk/λ
        let lambda_um = wavelength_nm / 1000.0;
        4.0 * std::f32::consts::PI * k / lambda_um
    }
}

/// Visible spectrum wavelengths
pub mod wavelengths {
    use super::Wavelength;

    // Ultraviolet spectrum (for steel rainbow)
    pub const DEEP_UV: Wavelength = 100.0;     // nm (extreme UV)
    pub const UV_C: Wavelength = 200.0;        // nm (far UV)
    pub const UV_B: Wavelength = 300.0;        // nm (mid UV)
    pub const UV_A: Wavelength = 380.0;        // nm (near UV)

    // UV range bounds
    pub const UV_MIN: Wavelength = DEEP_UV;    // 100 nm
    pub const UV_MAX: Wavelength = VIOLET;     // 400 nm

    // Visible spectrum
    pub const VIOLET: Wavelength = 400.0;  // nm
    pub const BLUE: Wavelength = 450.0;
    pub const CYAN: Wavelength = 500.0;
    pub const GREEN: Wavelength = 550.0;
    pub const YELLOW: Wavelength = 580.0;
    pub const ORANGE: Wavelength = 600.0;
    pub const RED: Wavelength = 700.0;

    // Infrared spectrum
    pub const NEAR_IR: Wavelength = 1000.0;    // 1 μm
    pub const MID_IR: Wavelength = 5000.0;     // 5 μm
    pub const FAR_IR: Wavelength = 10000.0;    // 10 μm

    /// Sample UV spectrum (100-400nm) - Steel rainbow region!
    /// This is where steel becomes transparent
    pub fn sample_uv(num_samples: usize) -> Vec<Wavelength> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / (num_samples - 1) as f32;
                DEEP_UV + t * (VIOLET - DEEP_UV)
            })
            .collect()
    }

    /// Sample visible spectrum with given number of points
    pub fn sample_visible(num_samples: usize) -> Vec<Wavelength> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / (num_samples - 1) as f32;
                VIOLET + t * (RED - VIOLET)
            })
            .collect()
    }

    /// Sample visible + infrared spectrum (400nm - 10μm)
    pub fn sample_visible_to_ir(num_samples: usize) -> Vec<Wavelength> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / (num_samples - 1) as f32;
                VIOLET + t * (FAR_IR - VIOLET)
            })
            .collect()
    }

    /// Sample infrared only (1-10 μm)
    pub fn sample_infrared(num_samples: usize) -> Vec<Wavelength> {
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / (num_samples - 1) as f32;
                NEAR_IR + t * (FAR_IR - NEAR_IR)
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
