/// Step 1.2: Ray-Circle Intersection Visualization
/// Renders rays and their intersection points with a circle
///
/// Run with: cargo run --example step_1_2_intersection

use iron_rainbow::{GpuContext, Ray, Circle, compute_intersections, Renderer2D};
use image::Rgb;

#[tokio::main]
async fn main() {
    println!("Step 1.2: Ray-Circle Intersection Visualization");
    println!("==============================================");

    // Initialize GPU
    let gpu = GpuContext::new().await;
    println!("GPU: {}", gpu.device_name());

    // Create test geometry
    let circle = Circle::new([0.0, 0.0], 1.0);

    // Create diverse test rays
    let test_rays = vec![
        // Cardinal directions (4 rays)
        Ray::new([-2.5, 0.0], [1.0, 0.0]),   // From left
        Ray::new([2.5, 0.0], [-1.0, 0.0]),   // From right
        Ray::new([0.0, 2.5], [0.0, -1.0]),   // From top
        Ray::new([0.0, -2.5], [0.0, 1.0]),   // From bottom

        // Diagonal rays (4 rays)
        Ray::normalized([-2.5, 2.5], [1.0, -1.0]),   // Top-left
        Ray::normalized([2.5, 2.5], [-1.0, -1.0]),   // Top-right
        Ray::normalized([2.5, -2.5], [-1.0, 1.0]),   // Bottom-right
        Ray::normalized([-2.5, -2.5], [1.0, 1.0]),   // Bottom-left

        // Grazing rays (2 rays)
        Ray::new([-2.5, 0.95], [1.0, 0.0]),  // Almost miss (top)
        Ray::new([-2.5, -0.95], [1.0, 0.0]), // Almost miss (bottom)

        // Miss rays (3 rays)
        Ray::new([-2.5, 2.0], [1.0, 0.0]),   // Parallel above
        Ray::new([-2.5, 0.0], [-1.0, 0.0]),  // Pointing away
        Ray::new([0.0, 0.0], [1.0, 0.0]),    // From center (behind)

        // Off-angle rays (3 rays)
        Ray::normalized([-2.5, 1.5], [1.0, -0.3]),
        Ray::normalized([1.5, 2.5], [-0.3, -1.0]),
        Ray::normalized([-2.5, -1.5], [1.0, 0.3]),
    ];

    println!("Testing {} rays against circle (radius={})...", test_rays.len(), circle.radius);

    // Compute intersections on GPU
    let results = compute_intersections(&gpu, &test_rays, &circle).await;
    println!("GPU computation complete");

    // Count hits
    let hit_count = results.iter().filter(|r| r.is_hit()).count();
    println!("Hits: {}/{}", hit_count, test_rays.len());

    // Create visualization
    println!("\nRendering visualization...");
    let mut renderer = Renderer2D::new(1200, 1200, 6.0);

    // Draw grid and axes
    renderer.draw_grid(1.0, Rgb([240, 240, 240]));
    renderer.draw_axes();

    // Draw circle (steel particle)
    renderer.draw_circle(
        circle.center[0],
        circle.center[1],
        circle.radius,
        Rgb([0, 0, 0])  // Black outline
    );
    renderer.draw_filled_circle(
        circle.center[0],
        circle.center[1],
        circle.radius,
        Rgb([220, 220, 230])  // Light steel color
    );

    // Draw rays and intersection points
    for (ray, result) in test_rays.iter().zip(results.iter()) {
        if result.is_hit() {
            let point = result.point();

            // Draw ray from origin to intersection (green)
            renderer.draw_arrow(
                ray.origin[0],
                ray.origin[1],
                point[0],
                point[1],
                Rgb([0, 180, 0])  // Green for hit
            );

            // Draw intersection point (red)
            renderer.draw_point(point[0], point[1], Rgb([255, 0, 0]));

            // Draw normal at intersection point (blue)
            let normal = circle.normal_at(point);
            renderer.draw_arrow(
                point[0],
                point[1],
                point[0] + normal[0] * 0.3,
                point[1] + normal[1] * 0.3,
                Rgb([0, 0, 255])  // Blue for normal
            );
        } else {
            // Draw missed ray (gray, shorter)
            let end_x = ray.origin[0] + ray.direction[0] * 2.0;
            let end_y = ray.origin[1] + ray.direction[1] * 2.0;
            renderer.draw_arrow(
                ray.origin[0],
                ray.origin[1],
                end_x,
                end_y,
                Rgb([150, 150, 150])  // Gray for miss
            );
        }

        // Draw ray origin (small black point)
        renderer.draw_filled_circle(
            ray.origin[0],
            ray.origin[1],
            0.03,
            Rgb([0, 0, 0])
        );
    }

    // Save image
    let output_path = "output/step_1_2_intersection.png";
    renderer.save(output_path).expect("Failed to save image");

    println!("✓ Visualization saved to: {}", output_path);
    println!("\nLegend:");
    println!("  Green arrows: Rays that hit the circle");
    println!("  Gray arrows: Rays that miss the circle");
    println!("  Red points: Intersection points");
    println!("  Blue arrows: Surface normals at intersection points");
    println!("  Black points: Ray origins");
}
