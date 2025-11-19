/// Physics validation tests for refraction calculations
/// These tests verify Snell's law is correctly implemented
use iron_rainbow::{compute_refractions, GpuContext, Ray, RefractionInput};

/// Test Snell's law: n1*sin(θ1) = n2*sin(θ2)
#[tokio::test]
async fn test_snells_law_air_to_glass() {
    let gpu = GpuContext::new().await;

    let n_air = 1.0_f32;
    let n_glass = 1.5_f32;

    // Test case: 30° incident angle from air to glass
    let incident_angle = 30.0_f32.to_radians();
    let expected_refracted_angle = (n_air * incident_angle.sin() / n_glass).asin();

    let ray = Ray::new([0.0, 0.0], [incident_angle.cos(), incident_angle.sin()]);
    let normal = [1.0, 0.0]; // Pointing right

    let input = RefractionInput::new(&ray, [0.0, 0.0], normal, n_air, n_glass);
    let results = compute_refractions(&gpu, &[input]).await;

    assert!(!results[0].is_tir(), "Should not be TIR");

    let refr = results[0].refracted_direction();
    let actual_angle = refr[1].atan2(refr[0]);

    let error = (actual_angle - expected_refracted_angle).abs();
    assert!(
        error < 0.01,
        "Snell's law violation: expected {:.3}°, got {:.3}°, error {:.3}°",
        expected_refracted_angle.to_degrees(),
        actual_angle.to_degrees(),
        error.to_degrees()
    );
}

/// Test critical angle and TIR
#[tokio::test]
async fn test_total_internal_reflection() {
    let gpu = GpuContext::new().await;

    let n_glass = 1.5_f32;
    let n_air = 1.0_f32;
    let critical_angle = (n_air / n_glass).asin();

    // Test 1: Below critical angle - should refract
    let below_critical = critical_angle - 0.1;
    let ray1 = Ray::new([0.0, 0.0], [below_critical.cos(), below_critical.sin()]);
    let input1 = RefractionInput::new(&ray1, [0.0, 0.0], [-1.0, 0.0], n_glass, n_air);
    let results1 = compute_refractions(&gpu, &[input1]).await;

    assert!(
        !results1[0].is_tir(),
        "Should refract at {:.1}° (below critical {:.1}°)",
        below_critical.to_degrees(),
        critical_angle.to_degrees()
    );

    // Test 2: Above critical angle - should have TIR
    let above_critical = critical_angle + 0.1;
    let ray2 = Ray::new([0.0, 0.0], [above_critical.cos(), above_critical.sin()]);
    let input2 = RefractionInput::new(&ray2, [0.0, 0.0], [-1.0, 0.0], n_glass, n_air);
    let results2 = compute_refractions(&gpu, &[input2]).await;

    assert!(
        results2[0].is_tir(),
        "Should have TIR at {:.1}° (above critical {:.1}°)",
        above_critical.to_degrees(),
        critical_angle.to_degrees()
    );
}

/// Test normal incidence (0°)
#[tokio::test]
async fn test_normal_incidence() {
    let gpu = GpuContext::new().await;

    let ray = Ray::new([0.0, 0.0], [1.0, 0.0]);
    let normal = [1.0, 0.0];

    let input = RefractionInput::new(&ray, [0.0, 0.0], normal, 1.0, 1.5);
    let results = compute_refractions(&gpu, &[input]).await;

    let refr = results[0].refracted_direction();

    // Should continue straight (no bending at normal incidence)
    let angle_error = refr[1].abs();
    assert!(
        angle_error < 0.01,
        "Normal incidence should go straight, but got angle {:.3}°",
        refr[1].atan2(refr[0]).to_degrees()
    );
}

/// Test refraction bends toward normal when entering denser medium
#[tokio::test]
async fn test_bends_toward_normal() {
    let gpu = GpuContext::new().await;

    let incident_angle = 45.0_f32.to_radians();
    let ray = Ray::new([0.0, 0.0], [incident_angle.cos(), incident_angle.sin()]);
    let normal = [1.0, 0.0];

    let input = RefractionInput::new(&ray, [0.0, 0.0], normal, 1.0, 1.5);
    let results = compute_refractions(&gpu, &[input]).await;

    let refr = results[0].refracted_direction();
    let refracted_angle = refr[1].atan2(refr[0]);

    assert!(
        refracted_angle.abs() < incident_angle.abs(),
        "Ray should bend toward normal: incident {:.1}°, refracted {:.1}°",
        incident_angle.to_degrees(),
        refracted_angle.to_degrees()
    );
}
