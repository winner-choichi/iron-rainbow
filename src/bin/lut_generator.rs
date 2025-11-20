use image::{ImageBuffer, Luma};
use iron_rainbow::{
    compute_path_traces,
    lut::{FalseColorStop, LutConfig},
    path_length_2d, Circle, DrudeModel, GpuContext, PathTraceInput, Ray,
};
use serde::Serialize;
use std::env;
use std::f32::consts::PI;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct LutMetadata {
    wavelength_min_nm: f32,
    wavelength_max_nm: f32,
    wavelength_values_nm: Vec<f32>,
    angle_min_deg: f32,
    angle_max_deg: f32,
    angle_steps: u32,
    normalize_mode: String,
    exposure: f32,
    droplet_radius_um: f32,
    impact_min: f32,
    impact_max: f32,
    rays_per_wavelength: u32,
    format: String,
    false_color: Vec<FalseColorStop>,
    channel_wavelengths: Option<Vec<f32>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let config_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("configs/lut_config.toml");

    let config = LutConfig::load(config_path)?;
    println!("LUT config: {}", config_path);

    let gpu = GpuContext::new().await;
    println!("GPU: {}", gpu.device_name());

    let steel = DrudeModel::steel();
    let wavelengths = config.wavelength_values();
    let num_rows = wavelengths.len();
    let num_cols = config.grid.angle_steps as usize;

    let mut grid = vec![vec![0.0f32; num_cols]; num_rows];
    let mut row_maxes = vec![0.0f32; num_rows];
    let mut global_max = 0.0f32;

    for (row_idx, wavelength) in wavelengths.iter().enumerate() {
        println!(
            "Simulating λ = {:.2} nm ({}/{})",
            wavelength,
            row_idx + 1,
            num_rows
        );
        let row_hist = simulate_wavelength(&gpu, &steel, &config, *wavelength).await;
        let row_max = row_hist.iter().cloned().fold(0.0f32, f32::max);
        row_maxes[row_idx] = row_max;
        if row_max > global_max {
            global_max = row_max;
        }
        grid[row_idx] = row_hist;
    }

    if global_max <= 0.0 {
        println!("Warning: no intensity generated. Defaulting normalization to 1.0");
        global_max = 1.0;
    }

    println!("\nLUT Statistics:");
    println!("  Global maximum intensity: {:.6}", global_max);
    println!("  Non-zero columns per row:");
    for (row_idx, row) in grid.iter().enumerate() {
        let non_zero = row.iter().filter(|&&v| v > 0.0).count();
        if non_zero > 0 {
            println!("    Row {}: {} / {} columns", row_idx, non_zero, row.len());
        }
    }

    let normalized = normalize_grid(&grid, &row_maxes, global_max, &config);
    save_texture(&normalized, &config)?;
    save_metadata(&config, &wavelengths)?;

    println!("✓ LUT saved to {}", config.output.texture_path);
    println!("✓ Metadata saved to {}", config.output.metadata_path);
    Ok(())
}

async fn simulate_wavelength(
    gpu: &GpuContext,
    steel: &DrudeModel,
    config: &LutConfig,
    wavelength: f32,
) -> Vec<f32> {
    let radius = config.droplet.radius_um;
    let circle = Circle::new([0.0, 0.0], radius);
    let n_air = 1.0;
    let (n_steel, k_steel) = steel.complex_index(wavelength);

    let ray_count = config.droplet.rays_per_wavelength.max(1) as usize;
    let mut inputs = Vec::with_capacity(ray_count);

    for i in 0..ray_count {
        let t = if ray_count <= 1 {
            0.5
        } else {
            i as f32 / (ray_count - 1) as f32
        };
        let impact_param =
            config.droplet.impact_min + t * (config.droplet.impact_max - config.droplet.impact_min);
        let b = impact_param * radius;
        let ray = Ray::new([-2.5 * radius, b], [1.0, 0.0]);
        inputs.push(PathTraceInput::new(&ray, &circle, n_air, n_steel));
    }

    let results = compute_path_traces(gpu, &inputs).await;
    let mut histogram = vec![0.0f32; config.grid.angle_steps as usize];
    let alpha = 4.0 * PI * k_steel / (wavelength * 0.001);

    for result in results.iter() {
        if result.num_events < 3 {
            continue;
        }

        let path1 = path_length_2d(result.event0_point, result.event1_point);
        let path2 = path_length_2d(result.event1_point, result.event2_point);
        let total_path = path1 + path2;
        let intensity = (-alpha * total_path).exp();
        if intensity <= 0.0 {
            continue;
        }

        // Calculate scattering angle (classical rainbow convention)
        let exit_dir = result.event2_direction;
        let incident_dir = [1.0, 0.0];
        let cos_theta = incident_dir[0] * exit_dir[0] + incident_dir[1] * exit_dir[1];
        let backward_angle = cos_theta.acos() * 180.0 / PI;

        // Convert to forward-equivalent angle (180° - θ)
        let scattering_angle = 180.0 - backward_angle;

        if let Some(exact_idx) = config.angle_index(scattering_angle) {
            let base = exact_idx.floor();
            let frac = exact_idx - base;
            let idx0 = base as usize;
            if idx0 < histogram.len() {
                histogram[idx0] += intensity * (1.0 - frac);
            }
            let idx1 = idx0 + 1;
            if idx1 < histogram.len() {
                histogram[idx1] += intensity * frac;
            }
        }
    }

    histogram
}

fn normalize_grid(
    grid: &Vec<Vec<f32>>,
    row_maxes: &Vec<f32>,
    global_max: f32,
    config: &LutConfig,
) -> Vec<Vec<f32>> {
    let exposure = config.intensity.exposure.max(0.0);
    let mut normalized = grid.clone();

    match config.intensity.normalize_mode.as_str() {
        "per_wavelength" => {
            for (row_idx, row) in normalized.iter_mut().enumerate() {
                let denom = if row_maxes[row_idx] <= 0.0 {
                    1.0
                } else {
                    row_maxes[row_idx]
                };
                for value in row.iter_mut() {
                    *value = ((*value / denom) * exposure).min(1.0);
                }
            }
        }
        _ => {
            let denom = if global_max <= 0.0 { 1.0 } else { global_max };
            for row in normalized.iter_mut() {
                for value in row.iter_mut() {
                    *value = ((*value / denom) * exposure).min(1.0);
                }
            }
        }
    }

    normalized
}

fn save_texture(
    data: &Vec<Vec<f32>>,
    config: &LutConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let height = data.len() as u32;
    let width = config.grid.angle_steps;

    if let Some(parent) = Path::new(&config.output.texture_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut img = ImageBuffer::<Luma<u16>, Vec<u16>>::from_raw(
        width,
        height,
        vec![0u16; (width as usize) * (height as usize)],
    )
    .expect("failed to allocate LUT texture");
    for (row_idx, row) in data.iter().enumerate() {
        for (col_idx, value) in row.iter().enumerate() {
            let clamped = value.clamp(0.0, 1.0);
            let val = (clamped * 65535.0).round() as u16;
            img.put_pixel(col_idx as u32, row_idx as u32, Luma([val]));
        }
    }

    img.save(&config.output.texture_path)?;
    Ok(())
}

fn save_metadata(
    config: &LutConfig,
    wavelengths: &Vec<f32>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = Path::new(&config.output.metadata_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let metadata = LutMetadata {
        wavelength_min_nm: config.grid.wavelength_min_nm,
        wavelength_max_nm: config.grid.wavelength_max_nm,
        wavelength_values_nm: wavelengths.clone(),
        angle_min_deg: config.grid.angle_min_deg,
        angle_max_deg: config.grid.angle_max_deg,
        angle_steps: config.grid.angle_steps,
        normalize_mode: config.intensity.normalize_mode.clone(),
        exposure: config.intensity.exposure,
        droplet_radius_um: config.droplet.radius_um,
        impact_min: config.droplet.impact_min,
        impact_max: config.droplet.impact_max,
        rays_per_wavelength: config.droplet.rays_per_wavelength,
        format: config.output.format.clone(),
        false_color: config.renderer.false_color.clone(),
        channel_wavelengths: config.renderer.channel_wavelengths.clone(),
    };

    let json = serde_json::to_string_pretty(&metadata)?;
    fs::write(&config.output.metadata_path, json)?;
    Ok(())
}
