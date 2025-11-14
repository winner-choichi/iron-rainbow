/// Step 1.6: Absorption Calculation (Beer-Lambert Law)
///
/// Demonstrates wavelength-dependent absorption through glass:
/// - Calculate path length through particle
/// - Apply Beer-Lambert law: I = I₀ × exp(-α × d)
/// - Visualize transmittance vs wavelength
///
/// Run with: cargo run --example step_1_6_absorption

use iron_rainbow::{
    beer_lambert, path_length_2d, transmittance, CauchyModel, Circle, DispersionModel,
    GpuContext, PathTraceInput, Renderer2D, compute_path_traces, wavelengths, Ray,
};
use image::Rgb;

#[tokio::main]
async fn main() {
    println!("Step 1.6: Absorption Calculation (Beer-Lambert Law)");
    println!("===================================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    // Material properties
    let glass = CauchyModel::glass();
    let circle = Circle::new([0.0, 0.0], 1.0);

    // Test ray with impact parameter b = 0.7
    let impact_param = 0.7;
    let b = impact_param * circle.radius;
    let ray = Ray::new([-2.5, b], [1.0, 0.0]);

    println!("Test ray: b/R = {}", impact_param);

    // Trace path
    let input = PathTraceInput::new(&ray, &circle, 1.0, 1.5);
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

    // Sample visible spectrum
    let num_wavelengths = 50;
    let test_wavelengths = wavelengths::sample_visible(num_wavelengths);

    // For glass, absorption is essentially zero in visible spectrum
    // We'll simulate a slightly absorbing material for demonstration
    let absorption_coeff = 0.0001;  // Very low absorption (glass is transparent)

    println!("Transmittance calculation:");
    println!("  Absorption coefficient: {} μm⁻¹", absorption_coeff);
    println!("  (Note: Real glass has ~0 absorption in visible)\n");

    // Calculate transmittance for each wavelength
    let transmittances: Vec<f32> = test_wavelengths
        .iter()
        .map(|_wl| transmittance(absorption_coeff, total_path))
        .collect();

    // Print some values
    println!("Sample transmittances:");
    for i in (0..test_wavelengths.len()).step_by(10) {
        println!("  λ = {:.0} nm: T = {:.4} ({:.1}%)",
                 test_wavelengths[i],
                 transmittances[i],
                 transmittances[i] * 100.0);
    }

    // Visualization
    let mut renderer = Renderer2D::new(1600, 1000, 10.0);

    let bg_color = Rgb([245, 250, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let curve_color = Rgb([255, 80, 0]);
    let axis_color = Rgb([100, 100, 100]);
    let grid_color = Rgb([220, 220, 220]);

    // Graph bounds
    let x_min = -4.5;
    let x_max = 4.5;
    let y_min = -4.5;
    let y_max = 4.5;

    // Map wavelength to x
    let wl_to_x = |wl: f32| {
        x_min + (wl - wavelengths::VIOLET) / (wavelengths::RED - wavelengths::VIOLET) * (x_max - x_min)
    };

    // Map transmittance to y
    let t_to_y = |t: f32| {
        y_min + t * (y_max - y_min)
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
    renderer.draw_thick_line(x_min, 0.0, x_max, 0.0, 0.03, axis_color);
    renderer.draw_thick_line(0.0, y_min, 0.0, y_max, 0.03, axis_color);

    // Draw transmittance curve
    for i in 0..test_wavelengths.len() - 1 {
        let x1 = wl_to_x(test_wavelengths[i]);
        let y1 = t_to_y(transmittances[i]);
        let x2 = wl_to_x(test_wavelengths[i + 1]);
        let y2 = t_to_y(transmittances[i + 1]);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.05, curve_color);
    }

    // Mark wavelengths
    let markers = [400.0, 500.0, 600.0, 700.0];
    for wl in markers.iter() {
        let x = wl_to_x(*wl);
        renderer.draw_thick_line(x, y_min, x, y_min + 0.2, 0.03, Rgb([0, 0, 0]));
    }

    // Mark transmittance values
    for i in 0..11 {
        let t = i as f32 / 10.0;
        let y = t_to_y(t);
        renderer.draw_thick_line(x_min, y, x_min + 0.2, y, 0.03, Rgb([0, 0, 0]));
    }

    let output_path = "output/step_1_6_absorption.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  X-axis: Wavelength (400-700 nm)");
    println!("  Y-axis: Transmittance T (0-1)");
    println!("  Orange curve: Beer-Lambert transmittance");
    println!("\nPhysics:");
    println!("  T = exp(-α × d)");
    println!("  For glass: α ≈ 0 → T ≈ 1 (nearly 100%)");
    println!("  For metals: α >> 1 → T ≈ 0 (opaque)");
    println!("\n✓ Step 1.6 complete!");
}
