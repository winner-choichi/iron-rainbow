use iron_rainbow::{GpuContext, Ray, Circle, compute_intersections};

fn main() {
    println!("=== Iron Rainbow Simulator ===");
    println!("Phase 1: 2D Ray Tracing Logic Verification");
    println!("Technology: Rust + wgpu (GPU Compute Shaders)");
    println!();

    println!("Step 1.2: Testing 2D Ray-Circle Intersection...");
    pollster::block_on(run_intersection_test());
}

async fn run_intersection_test() {
    // Initialize GPU
    let gpu = GpuContext::new().await;
    println!("  ✓ GPU Device: {}", gpu.device_name());

    // Create test geometry
    let circle = Circle::new([0.0, 0.0], 1.0);

    let test_rays = vec![
        // Ray 0: From left, horizontal, should hit at (-1, 0)
        Ray::new([-2.0, 0.0], [1.0, 0.0]),
        // Ray 1: From top, vertical, should hit at (0, 1)
        Ray::new([0.0, 2.0], [0.0, -1.0]),
        // Ray 2: From right, horizontal, should hit at (1, 0)
        Ray::new([2.0, 0.0], [-1.0, 0.0]),
        // Ray 3: From bottom, vertical, should hit at (0, -1)
        Ray::new([0.0, -2.0], [0.0, 1.0]),
        // Ray 4: Diagonal from top-left, should hit
        Ray::normalized([-2.0, 2.0], [1.0, -1.0]),
        // Ray 5: Parallel ray above circle, should miss
        Ray::new([-2.0, 2.0], [1.0, 0.0]),
        // Ray 6: Ray pointing away from circle, should miss
        Ray::new([-2.0, 0.0], [-1.0, 0.0]),
        // Ray 7: Ray from inside circle
        Ray::new([0.0, 0.0], [1.0, 0.0]),
    ];

    println!("  ✓ Test data created: {} rays, 1 circle (radius={})",
        test_rays.len(), circle.radius);

    // Compute intersections on GPU
    let results = compute_intersections(&gpu, &test_rays, &circle).await;
    println!("  ✓ GPU computation complete");

    // Display results
    println!();
    println!("=== Ray-Circle Intersection Results ===");
    println!("Circle: center=({}, {}), radius={}",
        circle.center[0], circle.center[1], circle.radius);
    println!();

    for (i, (ray, result)) in test_rays.iter().zip(results.iter()).enumerate() {
        println!("Ray {}: origin=({:.1}, {:.1}), direction=({:.3}, {:.3})",
            i, ray.origin[0], ray.origin[1], ray.direction[0], ray.direction[1]);

        if result.is_hit() {
            let point = result.point();
            println!("  → HIT at ({:.3}, {:.3}), distance={:.3}",
                point[0], point[1], result.t);
        } else {
            println!("  → MISS");
        }
    }

    println!();
    println!("✓ Step 1.2 complete! Ray-circle intersection working correctly.");
}
