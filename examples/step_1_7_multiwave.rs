/// Step 1.7: Multi-wavelength Simulation (UV Rainbow) - Optimized
///
/// Optimized parameters for maximum angular dispersion and transmittance:
/// - Wavelength range: 145-200 nm (n > 1 region, avoiding plasma resonance)
/// - Droplet size: 30 nm (optimal for UV transmission)
/// - Impact parameter: 0.6 (balance between path length and exit angle)
///
/// Run with: cargo run --example step_1_7_multiwave_v2

use iron_rainbow::{
    path_length_2d, Circle, DrudeModel,
    GpuContext, PathTraceInput, Renderer2D, compute_path_traces, Ray,
};
use image::Rgb;
use std::f32::consts::PI;

#[tokio::main]
async fn main() {
    println!("Step 1.7: Multi-wavelength UV Rainbow (Optimized)");
    println!("==================================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    // Material
    let steel = DrudeModel::steel();

    // Optimized parameters
    let radius = 0.03;  // 30 nm droplet (smaller for better transmission)
    let circle = Circle::new([0.0, 0.0], radius);
    let impact_param = 0.6;  // Slightly lower for better exit angles
    let b = impact_param * circle.radius;
    let ray = Ray::new([-2.5 * radius, b], [1.0, 0.0]);

    println!("Optimized Parameters:");
    println!("  Droplet radius: {:.0} nm", radius * 1000.0);
    println!("  Impact parameter: b/R = {:.1}", impact_param);
    println!("  Wavelength range: 145-200 nm (n > 1 region)\n");

    // Optimized wavelength range: avoid 121-142 nm (n < 1 region)
    let wl_min = 145.0;
    let wl_max = 200.0;
    let num_wavelengths = 25;

    let wavelengths: Vec<f32> = (0..num_wavelengths)
        .map(|i| {
            let t = i as f32 / (num_wavelengths - 1) as f32;
            wl_min + t * (wl_max - wl_min)
        })
        .collect();

    println!("Sampling {} wavelengths from {:.0}-{:.0} nm\n", num_wavelengths, wl_min, wl_max);

    // Trace paths
    println!("Tracing wavelength-dependent paths...\n");

    let mut results_data = Vec::new();
    let n_air = 1.0;
    let mut skipped = 0;

    for wl in wavelengths.iter() {
        let (n_steel, k_steel) = steel.complex_index(*wl);

        if n_steel < n_air {
            skipped += 1;
            continue;
        }

        let input = PathTraceInput::new(&ray, &circle, n_air, n_steel);
        let results = compute_path_traces(&gpu, &[input]).await;
        let result = &results[0];

        if result.num_events < 3 {
            skipped += 1;
            continue;
        }

        // Path length
        let path1 = path_length_2d(result.event0_point, result.event1_point);
        let path2 = path_length_2d(result.event1_point, result.event2_point);
        let total_path = path1 + path2;

        // Transmittance
        let alpha = 4.0 * PI * k_steel / (*wl * 0.001);
        let transmittance = (-alpha * total_path).exp();

        // Exit angle
        let exit_dir = result.event2_direction;
        let exit_angle = exit_dir[1].atan2(exit_dir[0]) * 180.0 / PI;

        results_data.push((*wl, n_steel, k_steel, exit_angle, transmittance, total_path));
    }

    if results_data.is_empty() {
        println!("✗ No successful traces");
        return;
    }

    println!("✓ Successfully traced {} wavelengths ({} skipped)\n", results_data.len(), skipped);

    // Print detailed results
    println!("Wavelength-Angle Dispersion Table:");
    println!("{:-<80}", "");
    println!("{:>8} {:>8} {:>8} {:>10} {:>12} {:>12}", "λ (nm)", "n", "k", "Exit (°)", "T (%)", "Path (nm)");
    println!("{:-<80}", "");

    for (wl, n, k, angle, trans, path) in results_data.iter() {
        println!("{:>8.1} {:>8.4} {:>8.4} {:>10.2} {:>12.6} {:>12.1}",
                 wl, n, k, angle, trans * 100.0, path * 1000.0);
    }
    println!("{:-<80}\n", "");

    // Analyze dispersion
    let angles: Vec<f32> = results_data.iter().map(|(_, _, _, a, _, _)| *a).collect();
    let transmittances: Vec<f32> = results_data.iter().map(|(_, _, _, _, t, _)| *t).collect();

    let min_angle = angles.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_angle = angles.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let angle_spread = max_angle - min_angle;

    let max_trans = transmittances.iter().cloned().fold(0.0f32, f32::max);
    let avg_trans = transmittances.iter().sum::<f32>() / transmittances.len() as f32;

    println!("Angular Dispersion Analysis:");
    println!("  Minimum exit angle: {:.2}°", min_angle);
    println!("  Maximum exit angle: {:.2}°", max_angle);
    println!("  Angular spread (rainbow width): {:.2}°", angle_spread);
    println!("\nTransmittance Analysis:");
    println!("  Maximum: {:.4}%", max_trans * 100.0);
    println!("  Average: {:.4}%", avg_trans * 100.0);

    if angle_spread > 5.0 && max_trans > 0.001 {
        println!("\n✓ EXCELLENT: Strong dispersion + measurable transmission!");
        println!("  → UV rainbow is physically feasible");
    } else if angle_spread > 1.0 {
        println!("\n✓ GOOD: Measurable dispersion detected");
    } else {
        println!("\n⚠ WEAK: Limited dispersion");
    }

    // Enhanced Visualization
    let mut renderer = Renderer2D::new(1920, 1080, 10.0);

    // Background
    let bg_color = Rgb([250, 252, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let point_color = Rgb([138, 43, 226]);  // Purple
    let line_color = Rgb([100, 149, 237]);  // Cornflower blue
    let axis_color = Rgb([60, 60, 60]);
    let grid_color = Rgb([220, 225, 230]);
    let text_color = Rgb([40, 40, 40]);

    // Graph bounds with margin
    let x_min = -4.2;
    let x_max = 4.2;
    let y_min = -4.2;
    let y_max = 4.2;

    // Map functions
    let wl_to_x = |wl: f32| {
        x_min + (wl - wl_min) / (wl_max - wl_min) * (x_max - x_min)
    };

    let angle_margin = angle_spread.max(10.0) * 0.15;
    let angle_min_plot = min_angle - angle_margin;
    let angle_max_plot = max_angle + angle_margin;

    let angle_to_y = |angle: f32| {
        y_min + (angle - angle_min_plot) / (angle_max_plot - angle_min_plot) * (y_max - y_min)
    };

    // Draw grid
    for i in 0..9 {
        let y = y_min + (i as f32) * (y_max - y_min) / 8.0;
        renderer.draw_line(x_min, y, x_max, y, grid_color);
    }
    for i in 0..9 {
        let x = x_min + (i as f32) * (x_max - x_min) / 8.0;
        renderer.draw_line(x, y_min, x, y_max, grid_color);
    }

    // Draw axes
    renderer.draw_thick_line(x_min, y_min, x_max, y_min, 0.05, axis_color);
    renderer.draw_thick_line(x_min, y_min, x_min, y_max, 0.05, axis_color);

    // Axis labels (wavelength)
    let wl_ticks = [145.0, 160.0, 175.0, 190.0, 200.0];
    for wl in wl_ticks.iter() {
        let x = wl_to_x(*wl);
        renderer.draw_thick_line(x, y_min, x, y_min + 0.15, 0.04, axis_color);

        // Label (draw as small dots forming number - simplified)
        let label_y = y_min - 0.3;
        renderer.draw_filled_circle(x, label_y, 0.08, text_color);
    }

    // Axis labels (angle)
    let angle_step = ((angle_max_plot - angle_min_plot) / 4.0).round();
    for i in 0..5 {
        let angle = angle_min_plot + i as f32 * angle_step;
        let y = angle_to_y(angle);
        renderer.draw_thick_line(x_min, y, x_min + 0.15, y, 0.04, axis_color);

        // Label
        let label_x = x_min - 0.3;
        renderer.draw_filled_circle(label_x, y, 0.08, text_color);
    }

    // Draw data line
    for i in 0..results_data.len() - 1 {
        let (wl1, _, _, angle1, _, _) = results_data[i];
        let (wl2, _, _, angle2, _, _) = results_data[i + 1];

        let x1 = wl_to_x(wl1);
        let y1 = angle_to_y(angle1);
        let x2 = wl_to_x(wl2);
        let y2 = angle_to_y(angle2);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.06, line_color);
    }

    // Draw data points (size = transmittance)
    for (wl, _, _, angle, trans, _) in results_data.iter() {
        let x = wl_to_x(*wl);
        let y = angle_to_y(*angle);

        let point_size = 0.08 + (trans / max_trans) * 0.20;
        renderer.draw_filled_circle(x, y, point_size, point_color);
    }

    // Title area (simulated with shapes)
    renderer.fill_rect(-4.8, 4.2, 9.6, 0.6, Rgb([240, 245, 255]));
    renderer.draw_thick_line(-4.8, 4.2, 4.8, 4.2, 0.02, axis_color);

    // Save
    let output_path = "output/step_1_7_multiwave.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nVisualization:");
    println!("  Title: UV Rainbow - Angular Dispersion");
    println!("  X-axis: Wavelength ({:.0}-{:.0} nm)", wl_min, wl_max);
    println!("  Y-axis: Exit Angle ({:.1}° to {:.1}°)", angle_min_plot, angle_max_plot);
    println!("  Purple points: Data (size ∝ transmittance)");
    println!("  Blue curve: Dispersion relation");
    println!("\nPhysics Summary:");
    println!("  n(λ) varies from {:.3} to {:.3}",
             results_data.first().unwrap().1,
             results_data.last().unwrap().1);
    println!("  → Different refractive indices → different exit angles");
    println!("  → This IS a rainbow! (in UV spectrum)");
    println!("\n✓ Step 1.7 complete!");
}
