// Snell's Law Refraction Compute Shader
// Implements refraction and total internal reflection

struct Ray {
    origin: vec2<f32>,
    direction: vec2<f32>,
}

struct Circle {
    center: vec2<f32>,
    radius: f32,
}

struct RefractionInput {
    ray_origin: vec2<f32>,
    ray_direction: vec2<f32>,
    intersection_point: vec2<f32>,
    normal: vec2<f32>,
    n1: f32,
    n2: f32,
    padding1: f32,
    padding2: f32,
}

struct RefractionResult {
    refracted_direction_x: f32,
    refracted_direction_y: f32,
    total_internal_reflection: f32,
    reflected_direction_x: f32,
    reflected_direction_y: f32,
    padding1: f32,
    padding2: f32,
    padding3: f32,
}

@group(0) @binding(0)
var<storage, read> inputs: array<RefractionInput>;

@group(0) @binding(1)
var<storage, read_write> results: array<RefractionResult>;

/// Compute refraction using Snell's Law
///
/// Snell's Law: n₁ sin(θ₁) = n₂ sin(θ₂)
///
/// Vector form refraction formula:
/// t = η·i + (η·cos(θ₁) - cos(θ₂))·n
///
/// where:
///   t = refracted ray direction (unit vector)
///   i = incident ray direction (unit vector)
///   n = surface normal (unit vector, pointing from n1 to n2)
///   η = n₁/n₂ (ratio of refractive indices)
///   θ₁ = angle of incidence
///   θ₂ = angle of refraction
///
/// Total Internal Reflection (TIR):
/// Occurs when sin(θ₂) > 1, which happens when:
///   η·sin(θ₁) > 1
/// or equivalently when discriminant < 0 in the formula below
fn compute_refraction(input: RefractionInput) -> RefractionResult {
    var result: RefractionResult;

    let i = input.ray_direction;   // Incident direction (normalized)
    let n = input.normal;          // Surface normal (normalized, pointing into incident medium)
    let eta = input.n1 / input.n2; // Ratio of refractive indices

    // Standard Snell's law refraction formula:
    // The normal should point OPPOSITE to incident ray for correct formula
    // cos(theta_i) = -dot(i, n) if normal points toward incident
    let cos_i = -dot(i, n);

    // Check which side we're on
    let normal_facing = select(n, -n, cos_i < 0.0);
    let cos_theta_i = abs(cos_i);

    // sin²(θt) = η² * sin²(θi) = η² * (1 - cos²(θi))
    let sin2_t = eta * eta * (1.0 - cos_theta_i * cos_theta_i);

    // Check for total internal reflection
    if (sin2_t > 1.0) {
        // Total Internal Reflection
        result.total_internal_reflection = 1.0;

        // Reflection: r = i - 2(i·n)n
        let reflected = i - 2.0 * dot(i, normal_facing) * normal_facing;
        result.reflected_direction_x = reflected.x;
        result.reflected_direction_y = reflected.y;

        result.refracted_direction_x = 0.0;
        result.refracted_direction_y = 0.0;
    } else {
        // Refraction
        result.total_internal_reflection = 0.0;

        let cos_theta_t = sqrt(1.0 - sin2_t);

        // Refracted direction: r = η*i + (η*cos(θi) - cos(θt))*n
        let refracted = eta * i + (eta * cos_theta_i - cos_theta_t) * normal_facing;

        result.refracted_direction_x = refracted.x;
        result.refracted_direction_y = refracted.y;

        // Reflection for Fresnel
        let reflected = i - 2.0 * dot(i, normal_facing) * normal_facing;
        result.reflected_direction_x = reflected.x;
        result.reflected_direction_y = reflected.y;
    }

    return result;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    results[index] = compute_refraction(inputs[index]);
}
