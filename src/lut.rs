use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct LutConfig {
    pub grid: GridConfig,
    pub intensity: IntensityConfig,
    pub droplet: DropletConfig,
    pub wavelength: WavelengthBatchConfig,
    pub output: OutputConfig,
    pub renderer: RendererConfig,
}

impl LutConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let text = fs::read_to_string(path)?;
        let config: LutConfig = toml::from_str(&text)?;
        Ok(config)
    }

    pub fn wavelength_values(&self) -> Vec<f32> {
        sample_range(
            self.grid.wavelength_min_nm,
            self.grid.wavelength_max_nm,
            self.grid.wavelength_steps.max(1),
        )
    }

    pub fn angle_index(&self, angle_deg: f32) -> Option<f32> {
        if self.grid.angle_max_deg <= self.grid.angle_min_deg {
            return Some(0.0);
        }
        if angle_deg < self.grid.angle_min_deg || angle_deg > self.grid.angle_max_deg {
            return None;
        }
        let normalized = (angle_deg - self.grid.angle_min_deg)
            / (self.grid.angle_max_deg - self.grid.angle_min_deg);
        let bins = (self.grid.angle_steps.max(1) - 1) as f32;
        Some(normalized * bins)
    }
}

fn sample_range(min: f32, max: f32, steps: u32) -> Vec<f32> {
    if steps <= 1 {
        return vec![min];
    }
    let mut values = Vec::with_capacity(steps as usize);
    let range = max - min;
    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        values.push(min + t * range);
    }
    values
}

#[derive(Debug, Deserialize, Clone)]
pub struct GridConfig {
    pub wavelength_min_nm: f32,
    pub wavelength_max_nm: f32,
    pub wavelength_steps: u32,
    pub angle_min_deg: f32,
    pub angle_max_deg: f32,
    pub angle_steps: u32,
    pub angle_supersample: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IntensityConfig {
    pub normalize_mode: String,
    pub exposure: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DropletConfig {
    pub radius_um: f32,
    pub impact_min: f32,
    pub impact_max: f32,
    pub impact_samples: u32,
    pub rays_per_wavelength: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WavelengthBatchConfig {
    pub batch_size: u32,
    pub wavelength_step_nm: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OutputConfig {
    pub texture_path: String,
    pub metadata_path: String,
    pub format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RendererConfig {
    pub false_color: Vec<FalseColorStop>,
    #[serde(default)]
    pub channel_wavelengths: Option<Vec<f32>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FalseColorStop {
    pub wavelength: f32,
    pub color: [u8; 3],
}
