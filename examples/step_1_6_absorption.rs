/// Step 1.6: Absorption Calculation (Beer-Lambert Law)
///
/// Demonstrates wavelength-dependent absorption through liquid steel (UV):
/// - Calculate path length through particle
/// - Apply Beer-Lambert law: I = I₀ × exp(-α × d)
/// - α = 4πk/λ where k is extinction coefficient from Drude model
/// - Visualize transmittance vs wavelength (UV spectrum 100-400nm)
///
/// Run with: cargo run --example step_1_6_absorption [radius_μm]
///   e.g. `cargo run --example step_1_6_absorption` (defaults to 1.0 μm)
///        `cargo run --example step_1_6_absorption -- 0.1` (100 nm droplet)

use iron_rainbow::{
    path_length_2d, Circle, DrudeModel,
    GpuContext, PathTraceInput, Renderer2D, compute_path_traces, wavelengths, Ray,
};
use image::Rgb;
use std::f32::consts::PI;

#[tokio::main]
async fn main() {
    println!("Step 1.6: Absorption Calculation (Beer-Lambert Law)");
    println!("===================================================\n");

    // Optional CLI argument: droplet radius in μm (default 1.0)
    let radius = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0);

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    // Material properties: Liquid Steel (Drude model)
    let steel = DrudeModel::steel();
    let circle = Circle::new([0.0, 0.0], radius);

    // Test ray with impact parameter b = 0.7
    let impact_param = 0.7;
    let b = impact_param * circle.radius;
    let ray = Ray::new([-2.5, b], [1.0, 0.0]);

    println!("Test ray: b/R = {}", impact_param);
    println!("Particle radius: {:.3} μm\n", circle.radius);

    // Get refractive index at 100nm (deep UV, below plasma wavelength)
    let test_wl = 100.0;
    let (n_steel, k_steel) = steel.complex_index(test_wl);

    println!("Steel optical constants at λ = {} nm:", test_wl);
    println!("  n_steel = {:.4}", n_steel);
    println!("  k_steel = {:.4} (extinction coefficient)", k_steel);

    // Surrounding medium: air (n = 1.0)
    let n_air = 1.0;
    println!("  n_air = {:.2} (surrounding medium)", n_air);
    println!("  n_rel = n_steel/n_air = {:.4}\n", n_steel / n_air);
    if n_steel > n_air {
        println!("  ✓ n_steel > n_air → Light refracts into steel particle!");
    } else {
        println!("  ⚠ n_steel < n_air → Total internal reflection risk");
    }

    // Trace path (air → steel)
    let input = PathTraceInput::new(&ray, &circle, n_air, n_steel);
    let results = compute_path_traces(&gpu, &[input]).await;
    let result = &results[0];

    if result.num_events < 3 {
        println!("Error: Path trace incomplete");
        return;
    }

    println!("Path traced successfully:");
    println!("  Entry:  ({:.3}, {:.3})", result.event0_point[0], result.event0_point[1]);
    println!("  Bounce: ({:.3}, {:.3})", result.event1_point[0], result.event1_point[1]);
    println!("  Exit:   ({:.3}, {:.3})\n", result.event2_point[0], result.event2_point[1]);

    // Calculate path lengths
    let path1 = path_length_2d(result.event0_point, result.event1_point);
    let path2 = path_length_2d(result.event1_point, result.event2_point);
    let total_path = path1 + path2;

    println!("Path lengths (inside particle):");
    println!("  Entry → Bounce: {:.3} μm", path1);
    println!("  Bounce → Exit:  {:.3} μm", path2);
    println!("  Total:          {:.3} μm\n", total_path);

    // Sample UV spectrum (100-400nm)
    let num_wavelengths = 100;
    let test_wavelengths = wavelengths::sample_uv(num_wavelengths);

    println!("Transmittance calculation (UV spectrum):");
    println!("  Beer-Lambert law: I = I₀ × exp(-α × d)");
    println!("  Absorption coefficient: α = 4πk/λ\n");

    // Calculate transmittance for each wavelength
    let transmittances: Vec<f32> = test_wavelengths
        .iter()
        .map(|wl| {
            let (_, k) = steel.complex_index(*wl);
            // α = 4πk/λ (convert λ from nm to μm)
            let alpha = 4.0 * PI * k / (*wl * 0.001);  // wl in nm → μm
            // Beer-Lambert: T = exp(-α × d)
            (-alpha * total_path).exp()
        })
        .collect();

    // Print some values
    println!("Sample transmittances (UV spectrum):");
    for i in (0..test_wavelengths.len()).step_by(10) {
        let (n, k) = steel.complex_index(test_wavelengths[i]);
        println!("  λ = {:.0} nm: n = {:.4}, k = {:.4}, T = {:.6} ({:.4}%)",
                 test_wavelengths[i],
                 n,
                 k,
                 transmittances[i],
                 transmittances[i] * 100.0);
    }

    // Find maximum transmittance for comparison
    let max_t = transmittances.iter().cloned().fold(0.0f32, f32::max);
    println!("\nMaximum transmittance: {:.6} ({:.4}%) at deep UV", max_t, max_t * 100.0);
    if max_t < 1e-4 {
        println!("  → With radius = {:.2} μm the particle is effectively opaque across 100-400nm", circle.radius);
        println!("  → Reduce particle radius (e.g., 0.05-0.10 μm) to explore non-zero transmittance\n");
    } else {
        println!("  → Non-zero transmission achieved; inspect curve for wavelength dependence\n");
    }

    // Visualization
    let mut renderer = Renderer2D::new(1920, 1080, 10.0);

    // Background
    let bg_color = Rgb([245, 250, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let curve_color = Rgb([138, 43, 226]);  // Purple for UV
    let axis_color = Rgb([100, 100, 100]);
    let grid_color = Rgb([220, 220, 220]);

    // Graph bounds
    let x_min = -4.5;
    let x_max = 4.5;
    let y_min = -4.5;
    let y_max = 4.5;

    // Map wavelength to x (UV range: 100-400nm)
    let wl_to_x = |wl: f32| {
        x_min + (wl - wavelengths::UV_MIN) / (wavelengths::UV_MAX - wavelengths::UV_MIN) * (x_max - x_min)
    };

    // Map transmittance to y (log scale for better visualization)
    let t_to_y = |t: f32| {
        if t < 1e-10 {
            y_min
        } else {
            let log_t = t.log10();  // -10 to 0
            y_min + (log_t + 10.0) / 10.0 * (y_max - y_min)
        }
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

    // Draw transmittance curve
    for i in 0..test_wavelengths.len() - 1 {
        let x1 = wl_to_x(test_wavelengths[i]);
        let y1 = t_to_y(transmittances[i]);
        let x2 = wl_to_x(test_wavelengths[i + 1]);
        let y2 = t_to_y(transmittances[i + 1]);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.05, curve_color);
    }

    // Mark wavelengths (UV range)
    let markers = [100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0];
    for wl in markers.iter() {
        let x = wl_to_x(*wl);
        renderer.draw_thick_line(x, y_min, x, y_min + 0.2, 0.03, Rgb([0, 0, 0]));
    }

    // Mark transmittance values (log scale: 10^-10 to 10^0)
    for i in 0..11 {
        let log_val = -10.0 + i as f32;  // -10, -9, ..., 0
        let t = 10.0_f32.powf(log_val);
        let y = t_to_y(t);
        renderer.draw_thick_line(x_min, y, x_min + 0.2, y, 0.03, Rgb([0, 0, 0]));
    }

    let output_path = "output/step_1_6_absorption.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  X-axis: Wavelength (100-400 nm, UV spectrum)");
    println!("  Y-axis: Transmittance T (log scale: 10^-10 to 10^0)");
    println!("  Purple curve: Beer-Lambert transmittance through steel");
    println!("\nPhysics:");
    println!("  T = exp(-α × d), where α = 4πk/λ");
    println!("  Larger droplets (R = {:.2} μm) yield long paths (d ≈ {:.3} μm) → exp(-αd) ≈ 0", circle.radius, total_path);
    println!("  Shrinking the droplet scales d ∝ R, so UV transmittance rises once αd ≲ 5");
    println!("\n✓ Step 1.6 complete!");
}
