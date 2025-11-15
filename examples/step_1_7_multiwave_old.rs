/// Step 1.7: Multi-wavelength Simulation (UV Rainbow)
///
/// Simulates wavelength-dependent ray paths through steel droplet:
/// - Sample UV spectrum (100-200 nm transparent region)
/// - Trace path for each wavelength with n(λ)
/// - Calculate exit angle for each wavelength
/// - Visualize angular dispersion (rainbow pattern!)
///
/// Run with: cargo run --example step_1_7_multiwave

use iron_rainbow::{
    path_length_2d, Circle, DrudeModel,
    GpuContext, PathTraceInput, Renderer2D, compute_path_traces, Ray,
};
use image::Rgb;
use std::f32::consts::PI;

#[tokio::main]
async fn main() {
    println!("Step 1.7: Multi-wavelength Simulation (UV Rainbow)");
    println!("===================================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    // Material properties
    let steel = DrudeModel::steel();

    // Small droplet for UV transmission
    let radius = 0.05;  // 50 nm droplet
    let circle = Circle::new([0.0, 0.0], radius);

    println!("Droplet radius: {:.3} μm ({:.0} nm)", radius, radius * 1000.0);
    println!("  → Small droplet for UV transmittance\n");

    // Impact parameter
    let impact_param = 0.7;
    let b = impact_param * circle.radius;
    let ray = Ray::new([-2.5 * radius, b], [1.0, 0.0]);

    println!("Test ray: b/R = {}\n", impact_param);

    // Sample UV spectrum (100-200 nm, transparent region)
    let num_wavelengths = 20;
    let wavelengths: Vec<f32> = (0..num_wavelengths)
        .map(|i| {
            let t = i as f32 / (num_wavelengths - 1) as f32;
            100.0 + t * 100.0  // 100-200 nm
        })
        .collect();

    println!("Sampling {} wavelengths from {:.0}-{:.0} nm\n",
             num_wavelengths, wavelengths[0], wavelengths[num_wavelengths-1]);

    // Trace paths for each wavelength
    println!("Tracing paths for each wavelength...");

    let mut results_data = Vec::new();
    let n_air = 1.0;

    for (i, wl) in wavelengths.iter().enumerate() {
        let (n_steel, k_steel) = steel.complex_index(*wl);

        if n_steel < n_air {
            println!("  λ = {:.0} nm: SKIP (n = {:.4} < 1, TIR risk)", wl, n_steel);
            continue;
        }

        // Trace path
        let input = PathTraceInput::new(&ray, &circle, n_air, n_steel);
        let results = compute_path_traces(&gpu, &[input]).await;
        let result = &results[0];

        if result.num_events < 3 {
            println!("  λ = {:.0} nm: FAIL (incomplete path)", wl);
            continue;
        }

        // Calculate path length
        let path1 = path_length_2d(result.event0_point, result.event1_point);
        let path2 = path_length_2d(result.event1_point, result.event2_point);
        let total_path = path1 + path2;

        // Calculate transmittance
        let alpha = 4.0 * PI * k_steel / (*wl * 0.001);
        let transmittance = (-alpha * total_path).exp();

        // Calculate exit angle
        let exit_dir = result.event2_direction;
        let exit_angle = exit_dir[1].atan2(exit_dir[0]) * 180.0 / PI;  // degrees

        results_data.push((*wl, n_steel, k_steel, exit_angle, transmittance, total_path));

        if i % 5 == 0 || i == num_wavelengths - 1 {
            println!("  λ = {:.0} nm: n = {:.4}, k = {:.4}, exit = {:.2}°, T = {:.4}%",
                     wl, n_steel, k_steel, exit_angle, transmittance * 100.0);
        }
    }

    if results_data.is_empty() {
        println!("\n✗ No successful traces. Try larger droplet or different wavelength range.");
        return;
    }

    println!("\n✓ Successfully traced {} wavelengths\n", results_data.len());

    // Analyze dispersion
    let angles: Vec<f32> = results_data.iter().map(|(_, _, _, angle, _, _)| *angle).collect();
    let min_angle = angles.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_angle = angles.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let angle_spread = max_angle - min_angle;

    println!("Angular Dispersion:");
    println!("  Min exit angle: {:.2}° (λ = {:.0} nm)", min_angle,
             results_data.iter().find(|(_, _, _, a, _, _)| (*a - min_angle).abs() < 0.01).unwrap().0);
    println!("  Max exit angle: {:.2}° (λ = {:.0} nm)", max_angle,
             results_data.iter().find(|(_, _, _, a, _, _)| (*a - max_angle).abs() < 0.01).unwrap().0);
    println!("  Angular spread: {:.2}° (rainbow width!)", angle_spread);

    if angle_spread < 0.1 {
        println!("  ⚠ Very small dispersion - n(λ) variation may be weak");
    } else {
        println!("  ✓ Measurable dispersion - UV rainbow detected!");
    }

    // Visualization: Exit angle vs Wavelength
    let mut renderer = Renderer2D::new(1920, 1080, 10.0);

    // Background
    let bg_color = Rgb([245, 250, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let point_color = Rgb([138, 43, 226]);  // Purple points
    let line_color = Rgb([100, 100, 255]);   // Blue line
    let axis_color = Rgb([100, 100, 100]);
    let grid_color = Rgb([220, 220, 220]);

    // Graph bounds
    let x_min = -4.5;
    let x_max = 4.5;
    let y_min = -4.5;
    let y_max = 4.5;

    // Map wavelength to x
    let wl_min = wavelengths[0];
    let wl_max = wavelengths[num_wavelengths - 1];
    let wl_to_x = |wl: f32| {
        x_min + (wl - wl_min) / (wl_max - wl_min) * (x_max - x_min)
    };

    // Map angle to y
    let angle_margin = angle_spread * 0.2;  // 20% margin
    let angle_min_plot = min_angle - angle_margin;
    let angle_max_plot = max_angle + angle_margin;
    let angle_to_y = |angle: f32| {
        y_min + (angle - angle_min_plot) / (angle_max_plot - angle_min_plot) * (y_max - y_min)
    };

    // Draw grid
    for i in 0..10 {
        let y = y_min + (i as f32) * (y_max - y_min) / 9.0;
        renderer.draw_line(x_min, y, x_max, y, grid_color);
    }
    for i in 0..10 {
        let x = x_min + (i as f32) * (x_max - x_min) / 9.0;
        renderer.draw_line(x, y_min, x, y_max, grid_color);
    }

    // Draw axes
    renderer.draw_thick_line(x_min, y_min, x_max, y_min, 0.03, axis_color);
    renderer.draw_thick_line(x_min, y_min, x_min, y_max, 0.03, axis_color);

    // Draw data points and connecting line
    for i in 0..results_data.len() {
        let (wl, _, _, angle, transmittance, _) = results_data[i];
        let x = wl_to_x(wl);
        let y = angle_to_y(angle);

        // Point size based on transmittance
        let point_size = 0.05 + transmittance * 0.15;

        // Draw point
        renderer.draw_filled_circle(x, y, point_size, point_color);

        // Connect to next point
        if i < results_data.len() - 1 {
            let (wl_next, _, _, angle_next, _, _) = results_data[i + 1];
            let x_next = wl_to_x(wl_next);
            let y_next = angle_to_y(angle_next);
            renderer.draw_thick_line(x, y, x_next, y_next, 0.02, line_color);
        }
    }

    // Mark wavelength ticks
    let wl_markers = [100.0, 120.0, 140.0, 160.0, 180.0, 200.0];
    for wl in wl_markers.iter() {
        if *wl >= wl_min && *wl <= wl_max {
            let x = wl_to_x(*wl);
            renderer.draw_thick_line(x, y_min, x, y_min + 0.2, 0.03, Rgb([0, 0, 0]));
        }
    }

    let output_path = "output/step_1_7_multiwave.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  X-axis: Wavelength ({:.0}-{:.0} nm, UV spectrum)", wl_min, wl_max);
    println!("  Y-axis: Exit angle ({:.2}°-{:.2}°)", angle_min_plot, angle_max_plot);
    println!("  Purple points: Each wavelength (size ∝ transmittance)");
    println!("  Blue line: Rainbow dispersion curve");
    println!("\nPhysics:");
    println!("  Different n(λ) → different exit angles → UV rainbow!");
    println!("  Droplet radius: {:.0} nm (small enough for UV transmission)", radius * 1000.0);
    println!("  Angular spread: {:.2}° (rainbow width)", angle_spread);
    println!("\n✓ Step 1.7 complete!");
}
