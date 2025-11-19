use image::Rgb;
/// Step 1.8: Massive Parallel Ray Tracing
///
/// GPU-accelerated simulation of thousands of rays with varying impact parameters:
/// - Generate rays with impact parameters from b/R = 0.0 to 0.9
/// - Parallel GPU computation for all rays simultaneously
/// - Build angular intensity distribution (histogram)
/// - Performance benchmarking
///
/// Run with: cargo run --release --example step_1_8_parallel_rays
use iron_rainbow::{
    compute_path_traces, path_length_2d, Circle, DrudeModel, GpuContext, PathTraceInput, Ray,
    Renderer2D,
};
use std::f32::consts::PI;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("Step 1.8: Massive Parallel Ray Tracing");
    println!("========================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    // Material
    let steel = DrudeModel::steel();

    // Droplet parameters (same as Step 1.7)
    let radius = 0.03; // 30 nm
    let circle = Circle::new([0.0, 0.0], radius);

    // Select single wavelength for initial test
    let wavelength = 170.0; // nm (peak n value from Step 1.7)
    let (n_steel, k_steel) = steel.complex_index(wavelength);
    let n_air = 1.0;

    println!("Test wavelength: {:.0} nm", wavelength);
    println!("  n = {:.4}, k = {:.4}\n", n_steel, k_steel);

    // Generate impact parameters: b/R from 0.05 to 0.85
    let num_rays = 100_000;
    let b_min = 0.05;
    let b_max = 0.85;

    println!(
        "Generating {} rays with impact parameters {} to {}",
        num_rays, b_min, b_max
    );
    println!("  → {} rays per GPU batch\n", num_rays);

    // Generate all input rays
    let start_gen = Instant::now();
    let inputs: Vec<PathTraceInput> = (0..num_rays)
        .map(|i| {
            let t = i as f32 / (num_rays - 1) as f32;
            let impact_param = b_min + t * (b_max - b_min);
            let b = impact_param * radius;
            let ray = Ray::new([-2.5 * radius, b], [1.0, 0.0]);
            PathTraceInput::new(&ray, &circle, n_air, n_steel)
        })
        .collect();
    let gen_time = start_gen.elapsed();

    println!(
        "✓ Generated {} ray inputs in {:.3} ms\n",
        inputs.len(),
        gen_time.as_secs_f64() * 1000.0
    );

    // GPU batch processing
    println!("Executing GPU path tracing...");
    let start_gpu = Instant::now();
    let results = compute_path_traces(&gpu, &inputs).await;
    let gpu_time = start_gpu.elapsed();

    println!(
        "✓ GPU traced {} rays in {:.3} ms",
        results.len(),
        gpu_time.as_secs_f64() * 1000.0
    );
    println!(
        "  → {:.1} million rays/sec\n",
        results.len() as f64 / gpu_time.as_secs_f64() / 1_000_000.0
    );

    // Analyze results: collect exit angles and intensities
    let start_analyze = Instant::now();

    let mut successful_rays = 0;
    let mut angle_intensity_pairs: Vec<(f32, f32)> = Vec::new();

    for (i, result) in results.iter().enumerate() {
        if result.num_events >= 3 {
            // Calculate transmittance
            let path1 = path_length_2d(result.event0_point, result.event1_point);
            let path2 = path_length_2d(result.event1_point, result.event2_point);
            let total_path = path1 + path2;

            let alpha = 4.0 * PI * k_steel / (wavelength * 0.001);
            let transmittance = (-alpha * total_path).exp();

            // Calculate scattering angle (classical rainbow convention)
            let exit_dir = result.event2_direction;
            let incident_dir = [1.0, 0.0]; // Horizontal right

            // cos(θ) = incident · exit
            let cos_theta = incident_dir[0] * exit_dir[0] + incident_dir[1] * exit_dir[1];
            let backward_angle = cos_theta.acos() * 180.0 / PI;

            // Convert to forward-equivalent angle (180° - θ)
            let scattering_angle = 180.0 - backward_angle;

            angle_intensity_pairs.push((scattering_angle, transmittance));
            successful_rays += 1;
        }
    }

    let analyze_time = start_analyze.elapsed();
    println!(
        "✓ Analyzed results in {:.3} ms",
        analyze_time.as_secs_f64() * 1000.0
    );
    println!(
        "  Success rate: {:.1}% ({}/{})\n",
        100.0 * successful_rays as f32 / num_rays as f32,
        successful_rays,
        num_rays
    );

    if angle_intensity_pairs.is_empty() {
        println!("✗ No successful rays");
        return;
    }

    // Build angular histogram (scattering angle: 0° to 180°)
    let angle_min = 0.0;
    let angle_max = 180.0;
    let num_bins = 180; // 1 degree per bin
    let bin_width = (angle_max - angle_min) / num_bins as f32;

    let mut histogram = vec![0.0f32; num_bins];

    for (angle, intensity) in angle_intensity_pairs.iter() {
        let bin_idx = ((angle - angle_min) / bin_width).floor() as i32;
        if bin_idx >= 0 && bin_idx < num_bins as i32 {
            histogram[bin_idx as usize] += intensity;
        }
    }

    // Find peak intensity and angle range
    let max_intensity = histogram.iter().cloned().fold(0.0f32, f32::max);
    let total_intensity: f32 = histogram.iter().sum();

    let angles: Vec<f32> = angle_intensity_pairs.iter().map(|(a, _)| *a).collect();
    let min_angle = angles.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_angle = angles.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    println!("Angular Distribution (Scattering Angle):");
    println!("  Angle range: {:.1}° to {:.1}°", min_angle, max_angle);
    println!("  Angular spread: {:.1}°", max_angle - min_angle);
    println!("  Peak intensity: {:.6}", max_intensity);
    println!("  Total intensity: {:.6}", total_intensity);
    println!("  Note: Water rainbow ≈ 42°, Alexander's dark band ≈ 50°\n");

    // Visualization
    println!("Generating visualization...");
    let mut renderer = Renderer2D::new(1920, 2160, 15.0);

    // Background
    let bg_color = Rgb([250, 252, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let curve_color = Rgb([220, 50, 50]); // Red for intensity
    let axis_color = Rgb([60, 60, 60]);
    let grid_color = Rgb([220, 225, 230]);
    let text_color = Rgb([40, 40, 40]);

    // Graph bounds
    let x_min = -4.2;
    let x_max = 4.2;
    let y_min = -4.2;
    let y_max = 4.2;

    // Map angle to x
    let angle_to_x =
        |angle: f32| x_min + (angle - min_angle) / (max_angle - min_angle) * (x_max - x_min);

    // Map intensity to y (linear scale)
    let intensity_to_y = |intensity: f32| y_min + (intensity / max_intensity) * (y_max - y_min);

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

    // Draw histogram
    for (i, intensity) in histogram.iter().enumerate() {
        if *intensity > 0.0 {
            let angle = angle_min + (i as f32 + 0.5) * bin_width;
            let x = angle_to_x(angle);
            let y = intensity_to_y(*intensity);

            // Draw vertical bar
            renderer.draw_thick_line(x, y_min, x, y, 0.02, curve_color);
        }
    }

    // Title
    let title_color = Rgb([40, 40, 40]);
    let title = format!("Angular Intensity Distribution (l={}nm)", wavelength);
    renderer.draw_text(-3.0, 5.2, &title, 0.28, title_color);

    // Axis labels
    renderer.draw_text(-1.5, -5.3, "Exit Angle (deg)", 0.22, axis_color);
    renderer.draw_text(-5.8, 0.0, "Intensity", 0.22, axis_color);

    // Angle axis ticks
    let angle_step = ((max_angle - min_angle) / 4.0).round();
    for i in 0..5 {
        let angle = min_angle + i as f32 * angle_step;
        let x = angle_to_x(angle);
        renderer.draw_thick_line(x, y_min, x, y_min + 0.15, 0.03, axis_color);
        let label = format!("{:.0}", angle);
        renderer.draw_text(x - 0.2, y_min - 0.45, &label, 0.18, text_color);
    }

    // Intensity axis ticks
    for i in 0..5 {
        let intensity = (i as f32 / 4.0) * max_intensity;
        let y = intensity_to_y(intensity);
        renderer.draw_thick_line(x_min, y, x_min + 0.15, y, 0.03, axis_color);
        let label = format!("{:.2}", intensity);
        renderer.draw_text(x_min - 0.7, y - 0.08, &label, 0.16, text_color);
    }

    let output_path = "output/step_1_8_parallel_rays.png";
    renderer.save(output_path).expect("Failed to save");

    println!("✓ Saved: {}\n", output_path);

    // Performance summary
    let total_time = gen_time + gpu_time + analyze_time;
    println!("Performance Summary:");
    println!(
        "  Ray generation:  {:.3} ms ({:.1}%)",
        gen_time.as_secs_f64() * 1000.0,
        100.0 * gen_time.as_secs_f64() / total_time.as_secs_f64()
    );
    println!(
        "  GPU computation: {:.3} ms ({:.1}%)",
        gpu_time.as_secs_f64() * 1000.0,
        100.0 * gpu_time.as_secs_f64() / total_time.as_secs_f64()
    );
    println!(
        "  Result analysis: {:.3} ms ({:.1}%)",
        analyze_time.as_secs_f64() * 1000.0,
        100.0 * analyze_time.as_secs_f64() / total_time.as_secs_f64()
    );
    println!(
        "  Total:           {:.3} ms",
        total_time.as_secs_f64() * 1000.0
    );
    println!(
        "  Throughput:      {:.1} million rays/sec\n",
        num_rays as f64 / total_time.as_secs_f64() / 1_000_000.0
    );

    println!("✓ Step 1.8 complete!");
}
