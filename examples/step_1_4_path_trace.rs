/// Step 1.4: Ray Path Tracing with Internal Reflection
///
/// Traces rays through a circular particle showing:
/// - Entry refraction
/// - Internal reflection (TIR)
/// - Exit refraction
///
/// Run with: cargo run --example step_1_4_path_trace

use iron_rainbow::{compute_path_traces, Circle, EventType, GpuContext, PathTraceInput, Ray, Renderer2D};
use image::Rgb;

#[tokio::main]
async fn main() {
    println!("Step 1.4: Ray Path Tracing with Internal Reflection");
    println!("===================================================\n");

    let gpu = GpuContext::new().await;
    println!("GPU: {}\n", gpu.device_name());

    let mut renderer = Renderer2D::new(1600, 1200, 6.0);

    // Colors
    let bg_color = Rgb([245, 250, 255]);
    let particle_color = Rgb([210, 230, 250]);
    let particle_outline = Rgb([100, 150, 200]);
    let incident_color = Rgb([255, 140, 0]);      // Orange
    let refracted_color = Rgb([0, 160, 255]);     // Cyan
    let reflected_color = Rgb([255, 40, 120]);    // Pink
    let exit_color = Rgb([100, 220, 100]);        // Green
    let point_color = Rgb([0, 0, 0]);             // Black

    // Fill background
    renderer.fill_rect(-3.0, 3.0, 6.0, 6.0, bg_color);

    // Particle (circle)
    let circle = Circle::new([0.0, 0.0], 1.0);

    // Draw particle (filled)
    for angle in 0..360 {
        let a = (angle as f32).to_radians();
        let r_inner = 0.0;
        let r_outer = circle.radius;
        for r_steps in 0..20 {
            let r = r_inner + (r_outer - r_inner) * (r_steps as f32 / 20.0);
            let x = circle.center[0] + r * a.cos();
            let y = circle.center[1] + r * a.sin();
            renderer.draw_pixel(x, y, particle_color);
        }
    }

    // Draw particle outline
    renderer.draw_circle(circle.center[0], circle.center[1], circle.radius, particle_outline);

    // Refractive indices
    let n_air = 1.0_f32;
    let n_glass = 1.5_f32;

    println!("Particle:");
    println!("  Center: ({}, {})", circle.center[0], circle.center[1]);
    println!("  Radius: {}", circle.radius);
    println!("  n_outside (air): {}", n_air);
    println!("  n_inside (glass): {}\n", n_glass);

    // Test rays at different impact parameters
    // Impact parameter b determines how far the ray is from center
    // For rainbow formation, we want b close to radius
    let impact_params = [0.2_f32, 0.4, 0.6, 0.7, 0.8, 0.9, 0.95];
    let mut test_cases = Vec::new();

    println!("Testing {} impact parameters:", impact_params.len());
    for (i, impact_param) in impact_params.iter().enumerate() {
        // Ray coming from the left, parallel to x-axis
        // Impact parameter = y coordinate of ray
        let b = impact_param * circle.radius;
        let ray_origin = [-2.5, b];
        let ray_direction = [1.0, 0.0];  // Horizontal ray
        let ray = Ray::new(ray_origin, ray_direction);

        test_cases.push((ray, *impact_param));
        println!("  Ray {}: b/R = {:.2}", i + 1, impact_param);
    }

    // Prepare inputs
    let inputs: Vec<PathTraceInput> = test_cases
        .iter()
        .map(|(ray, _)| PathTraceInput::new(ray, &circle, n_air, n_glass))
        .collect();

    println!("\nComputing {} path traces...", inputs.len());
    let results = compute_path_traces(&gpu, &inputs).await;
    println!("✓ Complete\n");

    // Visualize paths
    println!("Visualization:");
    for ((ray, impact_param), result) in test_cases.iter().zip(results.iter()) {
        println!("\nb/R = {:.2}:", impact_param);
        println!("  Events: {}", result.num_events);

        // Draw incident ray
        if result.has_event0() {
            renderer.draw_thick_line(
                ray.origin[0],
                ray.origin[1],
                result.event0_point[0],
                result.event0_point[1],
                0.05,
                incident_color,
            );
            renderer.draw_filled_circle(
                result.event0_point[0],
                result.event0_point[1],
                0.05,
                point_color,
            );

            println!("  Event 0: {:?} at ({:.3}, {:.3})",
                     result.event0_type(),
                     result.event0_point[0],
                     result.event0_point[1]);
        }

        // Draw refracted ray inside particle
        if result.has_event1() {
            renderer.draw_thick_line(
                result.event0_point[0],
                result.event0_point[1],
                result.event1_point[0],
                result.event1_point[1],
                0.05,
                refracted_color,
            );
            renderer.draw_filled_circle(
                result.event1_point[0],
                result.event1_point[1],
                0.05,
                point_color,
            );

            println!("  Event 1: {:?} at ({:.3}, {:.3})",
                     result.event1_type(),
                     result.event1_point[0],
                     result.event1_point[1]);
        }

        // Draw reflected ray inside particle (if TIR occurred)
        if result.has_event2() {
            let color = if result.event1_type() == EventType::TotalInternalReflection {
                reflected_color
            } else {
                refracted_color
            };

            renderer.draw_thick_line(
                result.event1_point[0],
                result.event1_point[1],
                result.event2_point[0],
                result.event2_point[1],
                0.05,
                color,
            );
            renderer.draw_filled_circle(
                result.event2_point[0],
                result.event2_point[1],
                0.05,
                point_color,
            );

            println!("  Event 2: {:?} at ({:.3}, {:.3})",
                     result.event2_type(),
                     result.event2_point[0],
                     result.event2_point[1]);

            // Draw exit ray
            renderer.draw_thick_line(
                result.event2_point[0],
                result.event2_point[1],
                result.event2_point[0] + result.event2_direction[0] * 1.0,
                result.event2_point[1] + result.event2_direction[1] * 1.0,
                0.05,
                exit_color,
            );
        }
    }

    let output_path = "output/step_1_4_path_trace.png";
    renderer.save(output_path).expect("Failed to save");

    println!("\n✓ Saved: {}", output_path);
    println!("\nLegend:");
    println!("  Orange: Incident ray");
    println!("  Cyan: Refracted ray (inside particle)");
    println!("  Pink: Reflected ray (TIR)");
    println!("  Green: Exit ray");
    println!("  Blue circle: Glass particle (n=1.5)");
    println!("\n✓ Step 1.4 complete!");
}
