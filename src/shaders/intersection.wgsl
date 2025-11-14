// 2D Ray-Circle Intersection Compute Shader
// Implements analytical ray-circle intersection algorithm

// Ray structure: origin (x, y) and direction (x, y)
struct Ray {
    origin: vec2<f32>,
    direction: vec2<f32>,
}

// Circle structure: center (x, y) and radius
struct Circle {
    center: vec2<f32>,
    radius: f32,
}

// Intersection result: hit flag, distance, and intersection point
struct IntersectionResult {
    hit: f32,      // 0.0 = no hit, 1.0 = hit
    t: f32,        // distance along ray to intersection
    point_x: f32,  // intersection point x coordinate
    point_y: f32,  // intersection point y coordinate
}

@group(0) @binding(0)
var<storage, read> rays: array<Ray>;

@group(0) @binding(1)
var<storage, read> circle: Circle;

@group(0) @binding(2)
var<storage, read_write> results: array<IntersectionResult>;

/// Ray-circle intersection using analytical solution
///
/// Ray equation: P(t) = origin + t * direction
/// Circle equation: |P - center|^2 = radius^2
///
/// Substituting ray into circle equation:
/// |origin + t*direction - center|^2 = radius^2
///
/// Expanding and rearranging into quadratic form: at^2 + bt + c = 0
/// where:
///   a = direction · direction
///   b = 2 * (origin - center) · direction
///   c = (origin - center) · (origin - center) - radius^2
///
/// Solution using discriminant (Δ = b^2 - 4ac):
///   Δ < 0: no intersection
///   Δ ≥ 0: t = (-b ± √Δ) / 2a
///          (take smaller t for nearest intersection)
fn ray_circle_intersect(ray: Ray, circle: Circle) -> IntersectionResult {
    var result: IntersectionResult;

    // Vector from circle center to ray origin
    let oc = ray.origin - circle.center;

    // Quadratic equation coefficients
    let a = dot(ray.direction, ray.direction);
    let b = 2.0 * dot(oc, ray.direction);
    let c = dot(oc, oc) - circle.radius * circle.radius;

    // Discriminant determines intersection existence
    let discriminant = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        // No intersection: ray misses circle
        result.hit = 0.0;
        result.t = -1.0;
        result.point_x = 0.0;
        result.point_y = 0.0;
    } else {
        // Intersection exists: compute nearest point
        let t = (-b - sqrt(discriminant)) / (2.0 * a);

        if (t >= 0.0) {
            // Valid intersection ahead of ray origin
            result.hit = 1.0;
            result.t = t;
            let point = ray.origin + t * ray.direction;
            result.point_x = point.x;
            result.point_y = point.y;
        } else {
            // Intersection behind ray origin (invalid)
            result.hit = 0.0;
            result.t = -1.0;
            result.point_x = 0.0;
            result.point_y = 0.0;
        }
    }

    return result;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;

    // Compute intersection for this ray
    results[index] = ray_circle_intersect(rays[index], circle);
}
