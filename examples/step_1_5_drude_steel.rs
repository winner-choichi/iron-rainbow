/// Step 1.5: Drude Model for Steel (UV Iron Rainbow)
///
/// Visualizes complex refractive index for liquid steel:
/// - n(λ): Real part (refraction)
/// - k(λ): Imaginary part (absorption)
/// - Range: UV (100-400nm) where steel becomes transparent
///
/// Discovery: Steel is transparent in UV (λ < plasma wavelength ~137nm)
/// → UV rainbow formation possible!
///
/// Run with: cargo run --example step_1_5_drude_steel

use iron_rainbow::{DrudeModel, Renderer2D, wavelengths};
use image::Rgb;

fn main() {
    println!("Step 1.5: Drude Model for Liquid Steel");
    println!("========================================\n");

    let steel = DrudeModel::steel();

    println!("Steel Drude parameters:");
    println!("  Plasma frequency (ωₚ): {:.2e} rad/s", steel.plasma_frequency);
    println!("  Damping (γ):           {:.2e} rad/s\n", steel.damping);

    // Sample wavelengths: UV spectrum (100-400nm)
    let num_samples = 200;
    let wavelengths_nm = wavelengths::sample_uv(num_samples);

    println!("Sampling {} wavelengths from {} nm (Deep UV) to {} nm (Near UV)\n",
             num_samples,
             wavelengths::DEEP_UV,
             wavelengths::VIOLET);

    // Calculate n and k for each wavelength
    let mut n_values = Vec::new();
    let mut k_values = Vec::new();

    for wl in &wavelengths_nm {
        let (n, k) = steel.complex_index(*wl);
        n_values.push(n);
        k_values.push(k);
    }

    // Print key wavelengths
    println!("Complex refractive index (n + ik):");
    println!("\nUV spectrum:");
    let test_uv = [100.0, 137.0, 200.0, 300.0, 380.0];
    for wl in test_uv.iter() {
        let (n, k) = steel.complex_index(*wl);
        let region = if *wl < 200.0 { "Deep UV" }
                    else if *wl < 280.0 { "UV-C" }
                    else if *wl < 315.0 { "UV-B" }
                    else { "UV-A" };
        println!("  λ = {} nm ({}): n = {:.3}, k = {:.3}", wl, region, n, k);
    }

    println!("\nVisible spectrum (for comparison):");
    let test_visible = [400.0, 500.0, 700.0];
    for wl in test_visible.iter() {
        let (n, k) = steel.complex_index(*wl);
        println!("  λ = {} nm: n = {:.3}, k = {:.3}", wl, n, k);
    }

    // Analyze UV transparency
    let (_n_uv, k_uv) = steel.complex_index(100.0);    // Deep UV
    let (_n_vis, k_vis) = steel.complex_index(500.0);  // Green light

    println!("\n✓ Discovery:");
    println!("  Deep UV (100nm): k = {:.3} (LOW → transparent!)", k_uv);
    println!("  Visible (500nm): k = {:.3} (HIGH → opaque)", k_vis);
    println!("  k ratio (UV/Vis): {:.6} (UV is {}x more transparent)",
             k_uv / k_vis, k_vis / k_uv);

    println!("\n  → Steel becomes transparent below plasma wavelength (~137nm)");
    println!("  → UV rainbow formation is possible!");

    // Visualization
    let mut renderer = Renderer2D::new(1600, 1200, 10.0);

    let bg_color = Rgb([245, 250, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let n_color = Rgb([0, 120, 255]);      // Blue for n
    let k_color = Rgb([255, 80, 0]);       // Orange for k
    let axis_color = Rgb([100, 100, 100]);
    let grid_color = Rgb([220, 220, 220]);

    // Graph bounds (linear scale for UV)
    let x_min = -4.5;
    let x_max = 4.5;
    let y_min = -4.5;
    let y_max = 4.5;

    // Map wavelength to x (linear scale)
    let wl_to_x = |wl: f32| {
        x_min + (wl - wavelengths::DEEP_UV) / (wavelengths::VIOLET - wavelengths::DEEP_UV) * (x_max - x_min)
    };

    // Find max n and k for scaling
    let max_n = n_values.iter().cloned().fold(0.0_f32, f32::max);
    let max_k = k_values.iter().cloned().fold(0.0_f32, f32::max);
    let y_scale = (y_max - y_min) / max_k.max(max_n).max(1.0);

    let val_to_y = |val: f32| y_min + val * y_scale;

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

    // Draw k(λ) curve (orange, absorption)
    for i in 0..wavelengths_nm.len() - 1 {
        let x1 = wl_to_x(wavelengths_nm[i]);
        let y1 = val_to_y(k_values[i]);
        let x2 = wl_to_x(wavelengths_nm[i + 1]);
        let y2 = val_to_y(k_values[i + 1]);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.05, k_color);
    }

    // Draw n(λ) curve (blue, refraction)
    for i in 0..wavelengths_nm.len() - 1 {
        let x1 = wl_to_x(wavelengths_nm[i]);
        let y1 = val_to_y(n_values[i]);
        let x2 = wl_to_x(wavelengths_nm[i + 1]);
        let y2 = val_to_y(n_values[i + 1]);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.05, n_color);
    }

    // Mark wavelength regions
    let markers = [
        (100.0, "100nm"),   // Deep UV
        (137.0, "λₚ"),      // Plasma wavelength
        (200.0, "200nm"),   // UV-C
        (300.0, "300nm"),   // UV-B
        (380.0, "380nm"),   // UV-A
    ];

    for (wl, _label) in markers.iter() {
        let x = wl_to_x(*wl);
        renderer.draw_thick_line(x, y_min, x, y_min + 0.3, 0.03, Rgb([0, 0, 0]));
    }

    let output_path = "output/step_1_5_drude_steel.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  Blue curve: n(λ) - Refractive index (real part)");
    println!("  Orange curve: k(λ) - Extinction coefficient (imaginary part)");
    println!("  X-axis: Wavelength (100-400nm, UV spectrum)");
    println!("  Y-axis: n or k value");
    println!("\nPhysics:");
    println!("  - k → 0 as λ → 100nm (below plasma wavelength)");
    println!("  - Steel becomes transparent in deep UV");
    println!("  - n(λ) dispersion → UV rainbow formation possible");
    println!("  - Note: UV rainbow is invisible to human eye!");
    println!("\n✓ Step 1.5 (Drude Model - UV) complete!");
}
