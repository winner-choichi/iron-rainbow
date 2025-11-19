use image::Rgb;
/// Step 1.9: Final Spectrogram (Angle vs Wavelength Heatmap)
///
/// 2D visualization of the complete UV rainbow:
/// - X-axis: Exit angle
/// - Y-axis: Wavelength
/// - Color: Intensity (transmittance-weighted)
///
/// This combines Step 1.7 (multi-wavelength) and Step 1.8 (parallel rays)
/// to create a comprehensive spectrogram of the steel rainbow.
///
/// Run with: cargo run --release --example step_1_9_spectrogram
use iron_rainbow::{
    compute_path_traces, path_length_2d, Circle, DrudeModel, GpuContext, PathTraceInput, Ray,
    Renderer2D,
};
use std::f32::consts::PI;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("Step 1.9: Spectrogram (Angle vs Wavelength)");
    println!("============================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    // Material
    let steel = DrudeModel::steel();

    // Droplet parameters (optimized from Step 1.7)
    let radius = 0.03; // 30 nm
    let circle = Circle::new([0.0, 0.0], radius);
    let n_air = 1.0;

    // Wavelength range (UV spectrum where n > 1)
    let wavelength_min = 145.0; // nm
    let wavelength_max = 200.0; // nm
    let num_wavelengths = 28; // Higher resolution for smoother heatmap

    // Impact parameter range
    let num_rays_per_wavelength = 1000; // Enough for good statistics
    let b_min = 0.05;
    let b_max = 0.85;

    println!("Configuration:");
    println!(
        "  Wavelength range: {:.0}-{:.0} nm ({} samples)",
        wavelength_min, wavelength_max, num_wavelengths
    );
    println!("  Rays per wavelength: {}", num_rays_per_wavelength);
    println!(
        "  Total rays: {}\n",
        num_wavelengths * num_rays_per_wavelength
    );

    // Data structures for 2D heatmap (scattering angle: 0° to 180°)
    let num_angle_bins = 180; // 1 degree resolution
    let angle_min = 0.0;
    let angle_max = 180.0; // Full scattering range
    let angle_bin_width = (angle_max - angle_min) / num_angle_bins as f32;

    // 2D grid: [wavelength_idx][angle_bin_idx] = intensity
    let mut heatmap: Vec<Vec<f32>> = vec![vec![0.0; num_angle_bins]; num_wavelengths];
    let mut wavelengths: Vec<f32> = Vec::new();

    let total_start = Instant::now();

    // Process each wavelength
    for wl_idx in 0..num_wavelengths {
        let t = wl_idx as f32 / (num_wavelengths - 1) as f32;
        let wavelength = wavelength_min + t * (wavelength_max - wavelength_min);
        wavelengths.push(wavelength);

        let (n_steel, k_steel) = steel.complex_index(wavelength);

        // Skip if n <= 1 (no refraction into droplet)
        if n_steel <= 1.0 {
            println!(
                "[{:2}/{}] λ={:.1}nm: n={:.4} ≤ 1, skipping",
                wl_idx + 1,
                num_wavelengths,
                wavelength,
                n_steel
            );
            continue;
        }

        // Generate rays for this wavelength
        let inputs: Vec<PathTraceInput> = (0..num_rays_per_wavelength)
            .map(|i| {
                let t = i as f32 / (num_rays_per_wavelength - 1) as f32;
                let impact_param = b_min + t * (b_max - b_min);
                let b = impact_param * radius;
                let ray = Ray::new([-2.5 * radius, b], [1.0, 0.0]);
                PathTraceInput::new(&ray, &circle, n_air, n_steel)
            })
            .collect();

        // GPU trace
        let results = compute_path_traces(&gpu, &inputs).await;

        // Accumulate into angle bins
        let mut successful = 0;
        for result in results.iter() {
            if result.num_events >= 3 {
                // Calculate transmittance
                let path1 = path_length_2d(result.event0_point, result.event1_point);
                let path2 = path_length_2d(result.event1_point, result.event2_point);
                let total_path = path1 + path2;

                let alpha = 4.0 * PI * k_steel / (wavelength * 0.001);
                let transmittance = (-alpha * total_path).exp();

                // Calculate scattering angle (classical rainbow convention)
                let exit_dir = result.event2_direction;
                let incident_dir = [1.0, 0.0];

                let cos_theta = incident_dir[0] * exit_dir[0] + incident_dir[1] * exit_dir[1];
                let backward_angle = cos_theta.acos() * 180.0 / PI;

                // Convert to forward-equivalent angle
                let scattering_angle = 180.0 - backward_angle;

                // Bin the angle
                if scattering_angle >= angle_min && scattering_angle <= angle_max {
                    let bin_idx =
                        ((scattering_angle - angle_min) / angle_bin_width).floor() as usize;
                    if bin_idx < num_angle_bins {
                        heatmap[wl_idx][bin_idx] += transmittance;
                        successful += 1;
                    }
                }
            }
        }

        println!(
            "[{:2}/{}] λ={:.1}nm: n={:.4}, k={:.4} → {} rays successful",
            wl_idx + 1,
            num_wavelengths,
            wavelength,
            n_steel,
            k_steel,
            successful
        );
    }

    let total_time = total_start.elapsed();
    println!("\n✓ Completed in {:.2} s", total_time.as_secs_f64());
    println!(
        "  Throughput: {:.1} million rays/sec\n",
        (num_wavelengths * num_rays_per_wavelength) as f64 / total_time.as_secs_f64() / 1_000_000.0
    );

    // Find global max for normalization
    let global_max = heatmap
        .iter()
        .flat_map(|row| row.iter())
        .cloned()
        .fold(0.0f32, f32::max);

    println!("Heatmap statistics:");
    println!("  Maximum intensity: {:.6}\n", global_max);

    // Visualization
    println!("Generating spectrogram...");
    let mut renderer = Renderer2D::new(1920, 2160, 15.0);

    // Background
    let bg_color = Rgb([250, 252, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let axis_color = Rgb([60, 60, 60]);
    let text_color = Rgb([40, 40, 40]);

    // Graph bounds
    let x_min = -4.2;
    let x_max = 4.2;
    let y_min = -4.2;
    let y_max = 4.2;

    // Mapping functions
    let angle_to_x =
        |angle: f32| x_min + (angle - angle_min) / (angle_max - angle_min) * (x_max - x_min);

    let wavelength_to_y = |wl: f32| {
        y_min + (wl - wavelength_min) / (wavelength_max - wavelength_min) * (y_max - y_min)
    };

    // Draw heatmap pixels
    let pixel_width = (x_max - x_min) / num_angle_bins as f32;
    let pixel_height = (y_max - y_min) / num_wavelengths as f32;

    for (wl_idx, row) in heatmap.iter().enumerate() {
        let wl = wavelengths[wl_idx];
        let y = wavelength_to_y(wl);

        for (angle_bin_idx, &intensity) in row.iter().enumerate() {
            if intensity > 0.0 {
                let angle = angle_min + (angle_bin_idx as f32 + 0.5) * angle_bin_width;
                let x = angle_to_x(angle);

                // Normalize intensity to [0, 1]
                let normalized = (intensity / global_max).min(1.0);

                // Color mapping: blue (low) → cyan → yellow → red (high)
                let color = intensity_to_color(normalized);

                renderer.fill_rect(x, y, pixel_width * 0.95, pixel_height * 0.95, color);
            }
        }
    }

    // Draw axes
    renderer.draw_thick_line(x_min, y_min, x_max, y_min, 0.05, axis_color);
    renderer.draw_thick_line(x_min, y_min, x_min, y_max, 0.05, axis_color);

    // Title
    renderer.draw_text(-2.5, 5.2, "UV Rainbow Spectrogram", 0.28, text_color);

    // Axis labels
    renderer.draw_text(-1.8, -5.3, "Scattering Angle (deg)", 0.22, axis_color);
    renderer.draw_text(-5.8, 0.0, "Wavelength (nm)", 0.22, axis_color);

    // X-axis ticks (scattering angles: 0° to 180°)
    let angle_ticks = vec![0.0, 20.0, 40.0, 60.0, 90.0, 120.0, 150.0, 180.0];
    for angle in angle_ticks.iter() {
        if *angle >= angle_min && *angle <= angle_max {
            let x = angle_to_x(*angle);
            renderer.draw_thick_line(x, y_min, x, y_min + 0.15, 0.03, axis_color);
            let label = if *angle == 42.0 {
                format!("{:.0}*", angle) // Mark water rainbow angle
            } else {
                format!("{:.0}", angle)
            };
            renderer.draw_text(x - 0.25, y_min - 0.45, &label, 0.18, text_color);
        }
    }

    // Y-axis ticks (wavelengths)
    let wl_step = 10.0; // Every 10 nm
    let mut wl_tick = (wavelength_min / wl_step).ceil() * wl_step;
    while wl_tick <= wavelength_max {
        let y = wavelength_to_y(wl_tick);
        renderer.draw_thick_line(x_min, y, x_min + 0.15, y, 0.03, axis_color);
        let label = format!("{:.0}", wl_tick);
        renderer.draw_text(x_min - 0.7, y - 0.08, &label, 0.16, text_color);
        wl_tick += wl_step;
    }

    // Color scale legend (optional enhancement)
    let legend_x = 3.0;
    let legend_y_min = -3.0;
    let legend_y_max = 3.0;
    let legend_width = 0.3;
    let legend_steps = 100;

    for i in 0..legend_steps {
        let t = i as f32 / (legend_steps - 1) as f32;
        let y = legend_y_min + t * (legend_y_max - legend_y_min);
        let color = intensity_to_color(t);
        renderer.fill_rect(
            legend_x,
            y,
            legend_width,
            (legend_y_max - legend_y_min) / legend_steps as f32,
            color,
        );
    }

    // Legend labels
    renderer.draw_text(legend_x + 0.4, legend_y_max - 0.1, "High", 0.16, text_color);
    renderer.draw_text(legend_x + 0.4, legend_y_min - 0.1, "Low", 0.16, text_color);

    let output_path = "output/step_1_9_spectrogram.png";
    renderer.save(output_path).expect("Failed to save");

    println!("✓ Saved: {}\n", output_path);
    println!("✓ Step 1.9 complete!");
    println!("\n🎉 Phase 1 COMPLETE! All micro-steps finished.");
}

/// Map normalized intensity [0,1] to color (blue → cyan → yellow → red)
fn intensity_to_color(t: f32) -> Rgb<u8> {
    let t = t.clamp(0.0, 1.0);

    // Color gradient: blue → cyan → green → yellow → red
    if t < 0.25 {
        // Blue to Cyan
        let s = t / 0.25;
        Rgb([0, (100.0 + s * 155.0) as u8, 255])
    } else if t < 0.5 {
        // Cyan to Green
        let s = (t - 0.25) / 0.25;
        Rgb([0, 255, (255.0 * (1.0 - s)) as u8])
    } else if t < 0.75 {
        // Green to Yellow
        let s = (t - 0.5) / 0.25;
        Rgb([(255.0 * s) as u8, 255, 0])
    } else {
        // Yellow to Red
        let s = (t - 0.75) / 0.25;
        Rgb([255, (255.0 * (1.0 - s)) as u8, 0])
    }
}
