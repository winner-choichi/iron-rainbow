use image::Rgb;
/// Step 1.5: Wavelength-Dependent Refractive Index
///
/// Visualizes dispersion curves for different materials:
/// - Cauchy model (glass)
/// - Sellmeier model (BK7 glass)
/// - Comparison of n(λ) across visible spectrum
///
/// Run with: cargo run --example step_1_5_dispersion
use iron_rainbow::{wavelengths, CauchyModel, DispersionModel, Renderer2D, SellmeierModel};

fn main() {
    println!("Step 1.5: Wavelength-Dependent Refractive Index");
    println!("===============================================\n");

    // Create dispersion models
    let cauchy_glass = CauchyModel::glass();
    let sellmeier_bk7 = SellmeierModel::bk7();

    // Sample visible spectrum
    let num_samples = 100;
    let wavelengths = wavelengths::sample_visible(num_samples);

    println!(
        "Sampling {} wavelengths from {} to {} nm\n",
        num_samples,
        wavelengths::VIOLET,
        wavelengths::RED
    );

    // Calculate refractive indices
    let cauchy_indices: Vec<f32> = wavelengths
        .iter()
        .map(|&wl| cauchy_glass.refractive_index(wl))
        .collect();

    let sellmeier_indices: Vec<f32> = wavelengths
        .iter()
        .map(|&wl| sellmeier_bk7.refractive_index(wl))
        .collect();

    // Print some key wavelengths
    println!("Cauchy Model (Simple Glass):");
    println!(
        "  Violet (400nm): n = {:.6}",
        cauchy_glass.refractive_index(400.0)
    );
    println!(
        "  Blue   (450nm): n = {:.6}",
        cauchy_glass.refractive_index(450.0)
    );
    println!(
        "  Green  (550nm): n = {:.6}",
        cauchy_glass.refractive_index(550.0)
    );
    println!(
        "  Yellow (580nm): n = {:.6}",
        cauchy_glass.refractive_index(580.0)
    );
    println!(
        "  Red    (700nm): n = {:.6}",
        cauchy_glass.refractive_index(700.0)
    );

    println!("\nSellmeier Model (BK7 Glass):");
    println!(
        "  Violet (400nm): n = {:.6}",
        sellmeier_bk7.refractive_index(400.0)
    );
    println!(
        "  Blue   (450nm): n = {:.6}",
        sellmeier_bk7.refractive_index(450.0)
    );
    println!(
        "  Green  (550nm): n = {:.6}",
        sellmeier_bk7.refractive_index(550.0)
    );
    println!(
        "  Yellow (580nm): n = {:.6}",
        sellmeier_bk7.refractive_index(580.0)
    );
    println!(
        "  Red    (700nm): n = {:.6}",
        sellmeier_bk7.refractive_index(700.0)
    );

    // Calculate dispersion (Δn = n_blue - n_red)
    let cauchy_dispersion =
        cauchy_glass.refractive_index(400.0) - cauchy_glass.refractive_index(700.0);
    let sellmeier_dispersion =
        sellmeier_bk7.refractive_index(400.0) - sellmeier_bk7.refractive_index(700.0);

    println!("\nDispersion (Δn = n_violet - n_red):");
    println!("  Cauchy:    Δn = {:.6}", cauchy_dispersion);
    println!("  Sellmeier: Δn = {:.6}", sellmeier_dispersion);

    // Visualization
    let mut renderer = Renderer2D::new(1600, 1000, 10.0);

    // Background
    let bg_color = Rgb([245, 250, 255]);
    renderer.fill_rect(-5.0, 5.0, 10.0, 10.0, bg_color);

    // Colors
    let cauchy_color = Rgb([0, 120, 255]); // Blue
    let sellmeier_color = Rgb([255, 80, 0]); // Orange
    let axis_color = Rgb([100, 100, 100]);
    let grid_color = Rgb([220, 220, 220]);

    // Graph bounds
    let x_min = -4.5;
    let x_max = 4.5;
    let y_min = -4.5;
    let y_max = 4.5;

    // Map wavelength to x coordinate
    let wl_to_x = |wl: f32| {
        x_min
            + (wl - wavelengths::VIOLET) / (wavelengths::RED - wavelengths::VIOLET)
                * (x_max - x_min)
    };

    // Map refractive index to y coordinate
    let n_min = 1.45;
    let n_max = 1.55;
    let n_to_y = |n: f32| y_min + (n - n_min) / (n_max - n_min) * (y_max - y_min);

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

    // Draw Cauchy curve
    for i in 0..wavelengths.len() - 1 {
        let x1 = wl_to_x(wavelengths[i]);
        let y1 = n_to_y(cauchy_indices[i]);
        let x2 = wl_to_x(wavelengths[i + 1]);
        let y2 = n_to_y(cauchy_indices[i + 1]);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.05, cauchy_color);
    }

    // Draw Sellmeier curve
    for i in 0..wavelengths.len() - 1 {
        let x1 = wl_to_x(wavelengths[i]);
        let y1 = n_to_y(sellmeier_indices[i]);
        let x2 = wl_to_x(wavelengths[i + 1]);
        let y2 = n_to_y(sellmeier_indices[i + 1]);

        renderer.draw_thick_line(x1, y1, x2, y2, 0.05, sellmeier_color);
    }

    // Mark key wavelengths
    let markers = [
        (400.0, "V"), // Violet
        (450.0, "B"), // Blue
        (550.0, "G"), // Green
        (580.0, "Y"), // Yellow
        (700.0, "R"), // Red
    ];

    for (wl, _label) in markers.iter() {
        let x = wl_to_x(*wl);
        renderer.draw_thick_line(x, y_min, x, y_min + 0.2, 0.03, Rgb([0, 0, 0]));
    }

    // Mark refractive index values
    for i in 0..11 {
        let n = n_min + (i as f32) * (n_max - n_min) / 10.0;
        let y = n_to_y(n);
        renderer.draw_thick_line(x_min, y, x_min + 0.2, y, 0.03, Rgb([0, 0, 0]));
    }

    let output_path = "output/step_1_5_dispersion.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  Blue curve: Cauchy model (simple glass)");
    println!("  Orange curve: Sellmeier model (BK7 glass)");
    println!("  X-axis: Wavelength (400-700 nm)");
    println!("  Y-axis: Refractive index n (1.45-1.55)");
    println!("\nPhysics:");
    println!("  - Shorter wavelengths (blue) have higher n → refract more");
    println!("  - This dispersion causes rainbow formation");
    println!("  - Δn ≈ 0.02 across visible spectrum");
    println!("\n✓ Step 1.5 complete!");
}
