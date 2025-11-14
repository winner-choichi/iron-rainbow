use image::Rgb;
/// Step 1.3: Snell's Law Refraction Visualization (Final Fixed)
///
/// Run with: cargo run --example step_1_3_refraction
use iron_rainbow::{compute_refractions, GpuContext, Ray, RefractionInput, Renderer2D};

#[tokio::main]
async fn main() {
    println!("Step 1.3: Snell's Law Refraction Visualization");
    println!("==============================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    let mut renderer = Renderer2D::new(1600, 7200, 8.0);

    // Colors
    let air_color = Rgb([230, 240, 255]);
    let glass_color = Rgb([210, 230, 250]);
    let interface_color = Rgb([60, 60, 60]);
    let incident_color = Rgb([255, 140, 0]);
    let refracted_color = Rgb([0, 160, 255]);
    let tir_color = Rgb([255, 40, 120]);
    let normal_color = Rgb([150, 150, 150]);

    // Fill background (optimized with fill_rect)
    renderer.fill_rect(-4.0, 4.0, 4.0, 8.0, air_color); // Left half (Air)
    renderer.fill_rect(0.0, 4.0, 4.0, 8.0, glass_color); // Right half (Glass)

    // Draw interface
    for i in -40..=40 {
        let y = i as f32 * 0.1;
        renderer.draw_filled_circle(0.0, y, 0.03, interface_color);
    }

    let n_air = 1.0_f32;
    let n_glass = 1.5_f32;
    let critical_angle = (n_air / n_glass).asin();

    println!("Air: n = {}", n_air);
    println!("Glass: n = {}", n_glass);
    println!("Critical angle: {:.1}°\n", critical_angle.to_degrees());

    // Test cases
    let mut test_cases = Vec::new();

    // ENTERING (Air → Glass)
    println!("=== Entering (Air → Glass) ===");
    for (i, angle_deg) in [0.0_f32, 15.0, 30.0, 45.0, 60.0].iter().enumerate() {
        let y_pos = 3.0 - (i as f32) * 1.5;
        let angle = angle_deg.to_radians();

        // Ray from left going right-down
        let ray_dir = [angle.cos(), -angle.sin()];
        let ray = Ray::new([-3.0, y_pos], ray_dir);

        // Intersection point at x=0
        let int_point = [0.0, y_pos - 3.0 * angle.tan()];

        // Normal points INTO the denser medium (to the right for entry)
        let normal = [1.0, 0.0];

        test_cases.push((ray, int_point, normal, n_air, n_glass, *angle_deg, "entry"));
    }

    // EXITING (Glass → Air)
    println!("=== Exiting (Glass → Air) ===");
    for (i, angle_deg) in [15.0_f32, 25.0, critical_angle.to_degrees(), 50.0, 70.0]
        .iter()
        .enumerate()
    {
        let y_pos = 3.0 - (i as f32) * 1.5;
        let angle = angle_deg.to_radians();

        // Ray from right going left-down
        let ray_dir = [-angle.cos(), -angle.sin()];
        let ray = Ray::new([3.0, y_pos], ray_dir);

        let int_point = [0.0, y_pos - 3.0 * angle.tan()];

        // Normal points INTO where ray is coming from (to the right for exit)
        let normal = [-1.0, 0.0];

        test_cases.push((ray, int_point, normal, n_glass, n_air, *angle_deg, "exit"));
    }

    // Compute refractions
    let inputs: Vec<RefractionInput> = test_cases
        .iter()
        .map(|(ray, int_pt, normal, n1, n2, _, _)| {
            RefractionInput::new(ray, *int_pt, *normal, *n1, *n2)
        })
        .collect();

    println!("\nComputing {} cases...", inputs.len());
    let results = compute_refractions(&gpu, &inputs).await;
    println!("✓ Complete\n");

    // Draw all rays
    for ((ray, int_point, _normal, _, _, angle_deg, mode), result) in
        test_cases.iter().zip(results.iter())
    {
        // 1. Draw INCIDENT ray (thick orange)
        renderer.draw_thick_line(
            ray.origin[0],
            ray.origin[1],
            int_point[0],
            int_point[1],
            0.08,
            incident_color,
        );

        // 2. Draw NORMAL (pointing toward incident side)
        let normal_dir = if *mode == "entry" {
            [-1.0, 0.0]
        } else {
            [1.0, 0.0]
        };
        renderer.draw_dashed_line(
            int_point[0],
            int_point[1],
            int_point[0] + normal_dir[0] * 0.6,
            int_point[1] + normal_dir[1] * 0.6,
            0.08,
            normal_color,
        );

        // 3. Draw REFRACTED or REFLECTED ray
        if result.is_tir() {
            println!("{}° ({}): TIR", angle_deg, mode);
            let refl = result.reflected_direction();

            // Thick pink for TIR
            renderer.draw_thick_line(
                int_point[0],
                int_point[1],
                int_point[0] + refl[0] * 2.5,
                int_point[1] + refl[1] * 2.5,
                0.08,
                tir_color,
            );
        } else {
            let refr = result.refracted_direction();
            println!("{}° ({}): → refracted", angle_deg, mode);

            // Thick cyan for refraction
            renderer.draw_thick_line(
                int_point[0],
                int_point[1],
                int_point[0] + refr[0] * 2.5,
                int_point[1] + refr[1] * 2.5,
                0.08,
                refracted_color,
            );
        }

        // 4. Mark intersection point
        renderer.draw_filled_circle(int_point[0], int_point[1], 0.08, Rgb([0, 0, 0]));
    }

    let output_path = "output/step_1_3_refraction.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  Orange rays: Incident");
    println!("  Cyan rays: Refracted");
    println!("  Pink rays: Total Internal Reflection (TIR)");
    println!("  Gray dashed: Surface normals");
    println!("  Blue background: Air (n=1.0)");
    println!("  Light blue background: Glass (n=1.5)");
    println!("\n✓ Step 1.3 complete!");
}
