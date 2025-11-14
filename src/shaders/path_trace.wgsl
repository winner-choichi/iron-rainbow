// Ray Path Tracing Compute Shader
// Traces a ray through a circular particle with internal reflection

struct Ray {
    origin: vec2<f32>,
    direction: vec2<f32>,
}

struct Circle {
    center: vec2<f32>,
    radius: f32,
}

// Input: Initial ray and particle properties
struct PathTraceInput {
    ray_origin: vec2<f32>,
    ray_direction: vec2<f32>,
    circle_center: vec2<f32>,
    circle_radius: f32,
    n_outside: f32,  // Refractive index outside (e.g., air = 1.0)
    n_inside: f32,   // Refractive index inside (e.g., glass = 1.5)
    padding1: f32,
    padding2: f32,
}

// Output: Ray path with up to 3 events (entry, internal reflection, exit)
struct PathTraceResult {
    // Event 0: Entry (refraction into particle)
    event0_point: vec2<f32>,
    event0_direction: vec2<f32>,
    event0_type: u32,  // 0=none, 1=refract, 2=reflect, 3=TIR
    _padding0: u32,

    // Event 1: Internal reflection
    event1_point: vec2<f32>,
    event1_direction: vec2<f32>,
    event1_type: u32,
    _padding1: u32,

    // Event 2: Exit (refraction out of particle)
    event2_point: vec2<f32>,
    event2_direction: vec2<f32>,
    event2_type: u32,
    _padding2: u32,

    num_events: u32,
    _padding3: u32,
}

@group(0) @binding(0)
var<storage, read> inputs: array<PathTraceInput>;

@group(0) @binding(1)
var<storage, read_write> results: array<PathTraceResult>;

// Ray-circle intersection (returns distance t, or -1 if no hit)
fn ray_circle_intersect(ray_origin: vec2<f32>, ray_direction: vec2<f32>,
                        circle_center: vec2<f32>, circle_radius: f32) -> vec2<f32> {
    let oc = ray_origin - circle_center;
    let a = dot(ray_direction, ray_direction);
    let b = 2.0 * dot(oc, ray_direction);
    let c = dot(oc, oc) - circle_radius * circle_radius;
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return vec2<f32>(-1.0, -1.0);
    }

    let sqrt_d = sqrt(discriminant);
    let t1 = (-b - sqrt_d) / (2.0 * a);
    let t2 = (-b + sqrt_d) / (2.0 * a);

    return vec2<f32>(t1, t2);
}

// Compute refraction using Snell's Law
// Returns: refracted direction (or vec2(0,0) if TIR)
// Also sets is_tir flag
fn compute_refraction(incident: vec2<f32>, normal: vec2<f32>,
                     eta: f32, is_tir: ptr<function, bool>) -> vec2<f32> {
    let cos_i = -dot(incident, normal);
    let normal_facing = select(-normal, normal, cos_i > 0.0);
    let cos_theta_i = abs(cos_i);

    let sin2_t = eta * eta * (1.0 - cos_theta_i * cos_theta_i);

    if (sin2_t > 1.0) {
        *is_tir = true;
        return vec2<f32>(0.0, 0.0);
    }

    *is_tir = false;
    let cos_theta_t = sqrt(1.0 - sin2_t);
    return eta * incident + (eta * cos_theta_i - cos_theta_t) * normal_facing;
}

// Compute reflection
fn compute_reflection(incident: vec2<f32>, normal: vec2<f32>) -> vec2<f32> {
    return incident - 2.0 * dot(incident, normal) * normal;
}

// Main path tracing function
fn trace_path(input: PathTraceInput) -> PathTraceResult {
    var result: PathTraceResult;
    result.num_events = 0u;
    result.event0_type = 0u;
    result.event1_type = 0u;
    result.event2_type = 0u;

    let ray_origin = input.ray_origin;
    let ray_direction = input.ray_direction;
    let circle_center = input.circle_center;
    let circle_radius = input.circle_radius;
    let n_out = input.n_outside;
    let n_in = input.n_inside;

    // Event 0: Entry into particle
    let t_entry = ray_circle_intersect(ray_origin, ray_direction, circle_center, circle_radius);
    if (t_entry.x < 0.0) {
        return result;  // No intersection
    }

    let entry_point = ray_origin + ray_direction * t_entry.x;
    let entry_normal = normalize(entry_point - circle_center);

    var is_tir = false;
    let refracted_dir = compute_refraction(ray_direction, entry_normal, n_out / n_in, &is_tir);

    if (is_tir) {
        // Should not happen for entry (air to glass)
        return result;
    }

    result.event0_point = entry_point;
    result.event0_direction = refracted_dir;
    result.event0_type = 1u;  // Refraction
    result.num_events = 1u;

    // Event 1: ALWAYS do internal reflection (for rainbow path)
    // Rainbow forms from 1-internal-reflection path
    let t_internal = ray_circle_intersect(entry_point + refracted_dir * 0.001,
                                          refracted_dir, circle_center, circle_radius);

    if (t_internal.y < 0.0) {
        return result;  // Should hit opposite side
    }

    let internal_point = entry_point + refracted_dir * t_internal.y;
    let internal_normal = normalize(internal_point - circle_center);

    // FORCE internal reflection (rainbow path)
    let reflected_dir = compute_reflection(refracted_dir, internal_normal);

    result.event1_point = internal_point;
    result.event1_direction = reflected_dir;
    result.event1_type = 2u;  // Reflection
    result.num_events = 2u;

    // Event 2: Exit after internal reflection
    let t_exit = ray_circle_intersect(internal_point + reflected_dir * 0.001,
                                     reflected_dir, circle_center, circle_radius);

    if (t_exit.y > 0.0) {
        let exit_point = internal_point + reflected_dir * t_exit.y;
        let exit_normal = normalize(exit_point - circle_center);

        var exit_tir = false;
        let final_refracted_dir = compute_refraction(reflected_dir, exit_normal, n_in / n_out, &exit_tir);

        if (!exit_tir) {
            result.event2_point = exit_point;
            result.event2_direction = final_refracted_dir;
            result.event2_type = 1u;  // Refraction
            result.num_events = 3u;
        }
    }

    return result;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= arrayLength(&inputs)) {
        return;
    }
    results[index] = trace_path(inputs[index]);
}
