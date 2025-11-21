// Rainbow Phase Function Shader
// Based on "Physically-Based Simulation of Rainbows" (SIGGRAPH 2012)
// Uses pre-computed phase function (LUT) for scattering angle -> intensity mapping

const MAX_FALSE_COLOR_STOPS: u32 = 16u;

struct ViewerUniform {
    camera_pos: vec3<f32>,
    _pad0: f32,
    camera_forward: vec3<f32>,
    _pad1: f32,
    camera_right: vec3<f32>,
    _pad2: f32,
    camera_up: vec3<f32>,
    _pad3: f32,
    sun_dir: vec3<f32>,
    _pad4: f32,
    droplet_center: vec3<f32>,
    droplet_radius: f32,
    viewport_width: f32,
    viewport_height: f32,
    fov: f32,
    exposure: f32,
    wavelength_min: f32,
    wavelength_range: f32,
    angle_min_deg: f32,
    angle_range_deg: f32,
    tex_width: u32,
    tex_height: u32,
    debug_mode: u32,
    march_steps: u32,
    channel_wavelengths: vec4<f32>,
    false_color_count: u32,
    _pad5: vec3<u32>,
    false_color_data: array<vec4<f32>, MAX_FALSE_COLOR_STOPS>,
}

@group(0) @binding(0) var lut_texture: texture_2d<f32>;
@group(0) @binding(1) var lut_sampler: sampler;
@group(0) @binding(2) var<uniform> params: ViewerUniform;
@group(0) @binding(3) var planet_texture: texture_2d<f32>;
@group(0) @binding(4) var planet_sampler: sampler;

const RAD_TO_DEG: f32 = 57.29577951;
const DEG_TO_RAD: f32 = 0.017453293;
const PI: f32 = 3.14159265359;

fn intersect_plane(ro: vec3<f32>, rd: vec3<f32>, normal: vec3<f32>, d: f32) -> f32 {
    let denom = dot(normal, rd);
    if (abs(denom) < 1e-5) {
        return -1.0;
    }
    let t = -(dot(normal, ro) + d) / denom;
    if (t > 0.0) {
        return t;
    }
    return -1.0;
}

// Procedural hash for 3D space (no spherical distortion)
fn hash3(p: vec3<f32>) -> f32 {
    let p3 = fract(p * 0.1031);
    let p3_dot = dot(p3, vec3<f32>(p3.y + 19.19, p3.z + 19.19, p3.x + 19.19));
    return fract((p3.x + p3.y + p3.z) * p3_dot);
}

fn stars(dir: vec3<f32>) -> vec3<f32> {
    // Sample stars directly in 3D space (uniform on sphere)
    let scale = 500.0; // Higher scale = smaller, denser stars
    let grid_pos = floor(dir * scale);

    // Generate star at this grid cell
    let star_hash = hash3(grid_pos);

    // Very sparse stars (higher threshold = fewer stars)
    if (star_hash > 0.9995) {
        // Small, sharp point-like stars
        let brightness = (star_hash - 0.9995) * 2000.0; // Bright but small

        // Slight color variation
        let color_var = hash3(grid_pos * 1.234);
        if (color_var < 0.15) {
            return vec3<f32>(brightness * 0.7, brightness * 0.85, brightness); // Blue
        } else if (color_var < 0.3) {
            return vec3<f32>(brightness, brightness * 0.95, brightness * 0.8); // Yellow
        } else {
            return vec3<f32>(brightness); // White
        }
    }

    return vec3<f32>(0.0);
}

// Ground surface with high-res local texture mapping
fn ground_surface(world_pos: vec3<f32>) -> vec3<f32> {
    // Map texture to local region around spawn point (origin)
    // Cover hemisphere: 30km × 30km centered at origin (planet radius = 10km)
    let texture_coverage = 30000.0; // 30km

    // Local planar UV mapping (high resolution)
    let u = (world_pos.x + texture_coverage * 0.5) / texture_coverage;
    let v = (world_pos.z + texture_coverage * 0.5) / texture_coverage;

    let uv = vec2<f32>(u, v);

    // Check if we're inside texture coverage area
    let in_coverage = u >= 0.0 && u <= 1.0 && v >= 0.0 && v <= 1.0;

    var tex_color: vec3<f32>;
    if (in_coverage) {
        // High-res texture in local area
        tex_color = textureSample(planet_texture, planet_sampler, uv).rgb;
    } else {
        // Fallback: dark gray for areas beyond texture coverage
        let base_color = vec3<f32>(0.12, 0.13, 0.15);
        let noise_val = hash3(vec3<f32>(world_pos.x * 0.001, 0.0, world_pos.z * 0.001));
        tex_color = base_color * (0.8 + noise_val * 0.4);
    }

    // Distance-based fog
    let dist = length(world_pos - params.camera_pos);
    let fade = clamp(1.0 - dist / 100000.0, 0.3, 1.0);

    return tex_color * fade;
}

// Sample phase function (LUT) for given scattering angle and wavelength
fn sample_phase_function(theta_deg: f32, wavelength: f32) -> f32 {
    if (params.angle_range_deg <= 0.0 || params.wavelength_range <= 0.0) {
        return 0.0;
    }

    // Explicit range check: return 0 if outside rainbow angle range
    let angle_max = params.angle_min_deg + params.angle_range_deg;
    if (theta_deg < params.angle_min_deg || theta_deg > angle_max) {
        return 0.0;
    }

    let tex_w = max(f32(params.tex_width), 1.0);
    let tex_h = max(f32(params.tex_height), 1.0);

    // Map scattering angle to LUT coordinate
    let angle_norm = clamp((theta_deg - params.angle_min_deg) / params.angle_range_deg, 0.0, 1.0);
    let wl_norm = clamp((wavelength - params.wavelength_min) / params.wavelength_range, 0.0, 1.0);

    let u = (angle_norm * (tex_w - 1.0) + 0.5) / tex_w;
    let v = (wl_norm * (tex_h - 1.0) + 0.5) / tex_h;

    let intensity = textureSample(lut_texture, lut_sampler, vec2<f32>(u, v)).r;
    return intensity;
}

// Map wavelength to RGB color using false_color gradient
fn wavelength_to_color(wavelength: f32) -> vec3<f32> {
    if (params.false_color_count == 0u) {
        return vec3<f32>(1.0);
    }
    
    if (params.false_color_count == 1u) {
        return params.false_color_data[0].yzw;
    }
    
    // Find the two stops that bracket this wavelength
    var lower_idx = 0u;
    var upper_idx = 1u;
    
    // If below first stop, extrapolate from first two stops
    if (wavelength <= params.false_color_data[0].x) {
        lower_idx = 0u;
        upper_idx = 1u;
    }
    // If above last stop, extrapolate from last two stops
    else if (wavelength >= params.false_color_data[params.false_color_count - 1u].x) {
        lower_idx = params.false_color_count - 2u;
        upper_idx = params.false_color_count - 1u;
    }
    // Find bracketing stops
    else {
        for (var i = 0u; i < params.false_color_count - 1u; i = i + 1u) {
            if (params.false_color_data[i].x <= wavelength && wavelength <= params.false_color_data[i + 1u].x) {
                lower_idx = i;
                upper_idx = i + 1u;
                break;
            }
        }
    }
    
    let lower_stop = params.false_color_data[lower_idx];
    let upper_stop = params.false_color_data[upper_idx];
    
    // Linear interpolation (or extrapolation)
    let t = (wavelength - lower_stop.x) / (upper_stop.x - lower_stop.x);
    return mix(lower_stop.yzw, upper_stop.yzw, t);
}

@vertex fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Fullscreen triangle
    let x = f32((vertex_index & 1u) << 2u) - 1.0;
    let y = 1.0 - f32((vertex_index & 2u) << 1u);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Debug mode 9: Test pattern
    if (params.debug_mode == 9u) {
        let uv = vec2<f32>(coord.x / params.viewport_width, coord.y / params.viewport_height);
        return vec4<f32>(uv.x, uv.y, 0.5, 1.0);
    }

    // Normalized device coordinates
    let ndc = vec2<f32>(
        (coord.x / params.viewport_width) * 2.0 - 1.0,
        1.0 - (coord.y / params.viewport_height) * 2.0
    );

    let aspect = params.viewport_width / params.viewport_height;

    // Generate camera ray (viewing direction)
    let fov_scale = tan(params.fov * 0.5);
    let ray_dir_cam = normalize(vec3<f32>(
        ndc.x * aspect * fov_scale,
        ndc.y * fov_scale,
        1.0
    ));

    let ray_dir = normalize(
        ray_dir_cam.x * params.camera_right +
        ray_dir_cam.y * params.camera_up +
        ray_dir_cam.z * params.camera_forward
    );

    // Debug mode 8: Ray direction
    if (params.debug_mode == 8u) {
        return vec4<f32>(abs(ray_dir), 1.0);
    }

    // Rainbow physics: Angle from anti-solar point
    // Anti-solar point = opposite direction of sun
    // Rainbow forms at specific angles from this point
    // Water rainbow ≈ 42°, Iron rainbow ≈ 0.5-52° (our simulation)
    let anti_solar = -params.sun_dir;
    let cos_theta = dot(ray_dir, anti_solar);
    let theta_rad = acos(clamp(cos_theta, -1.0, 1.0));
    let scattering_angle = theta_rad * RAD_TO_DEG;

    // Debug mode 7: Scattering angle visualization
    if (params.debug_mode == 7u) {
        let norm = scattering_angle / 180.0;
        return vec4<f32>(norm, norm, norm, 1.0);
    }

    // Debug mode 6: Show expected rainbow range (0° - 52°)
    if (params.debug_mode == 6u) {
        if (scattering_angle >= 0.0 && scattering_angle <= 52.0) {
            let norm = scattering_angle / 52.0;
            return vec4<f32>(0.0, norm, 1.0 - norm, 1.0);
        } else {
            return vec4<f32>(0.1, 0.1, 0.1, 1.0);
        }
    }

    // LUT lookup - scattering angle directly maps to LUT (0° to 180°)
    let lut_angle = scattering_angle;

    // Debug mode 5: LUT range check
    if (params.debug_mode == 5u) {
        if (lut_angle >= params.angle_min_deg &&
            lut_angle <= params.angle_min_deg + params.angle_range_deg) {
            return vec4<f32>(0.0, 1.0, 0.0, 1.0); // Green = in range
        } else {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red = out of range
        }
    }

    // Pre-sample reference channels (debug + fallback)
    let channels = params.channel_wavelengths;
    let intensity_r = sample_phase_function(lut_angle, channels.x);
    let intensity_g = sample_phase_function(lut_angle, channels.y);
    let intensity_b = sample_phase_function(lut_angle, channels.z);

    // Debug modes 1-3: Individual channels
    if (params.debug_mode == 1u) {
        return vec4<f32>(intensity_r, 0.0, 0.0, 1.0) * params.exposure;
    }
    if (params.debug_mode == 2u) {
        return vec4<f32>(0.0, intensity_g, 0.0, 1.0) * params.exposure;
    }
    if (params.debug_mode == 3u) {
        return vec4<f32>(0.0, 0.0, intensity_b, 1.0) * params.exposure;
    }
    
    // Debug mode 4: Show spectral integration raw output
    if (params.debug_mode == 4u) {
        var debug_color = vec3<f32>(0.0);
        if (params.false_color_count > 0u) {
            let num_samples = 32u;
            for (var i = 0u; i < num_samples; i = i + 1u) {
                let t = (f32(i) + 0.5) / f32(num_samples);
                let wavelength = params.wavelength_min + t * params.wavelength_range;
                let intensity = sample_phase_function(lut_angle, wavelength);
                let wl_color = wavelength_to_color(wavelength);
                debug_color += intensity * wl_color;
            }
        }
        return vec4<f32>(debug_color * params.exposure, 1.0);
    }


    // Spectral integration rendering
    var color = vec3<f32>(0.0);

    if (params.false_color_count > 0u) {
        // Render with smooth spectral integration (32 samples)
        let num_samples = 32u;
        
        for (var i = 0u; i < num_samples; i = i + 1u) {
            let t = (f32(i) + 0.5) / f32(num_samples);
            let wavelength = params.wavelength_min + t * params.wavelength_range;
            let intensity = sample_phase_function(lut_angle, wavelength);
            let wl_color = wavelength_to_color(wavelength);
            color += intensity * wl_color;
        }
    } else {
        // Fallback to RGB channels if no false color defined
        color = vec3<f32>(intensity_r, intensity_g, intensity_b);
    }

    // Apply exposure
    color *= params.exposure;
    
    // Angle-based fade-out on both sides
    let fade_start_angle = 25.0;  // Inner fade (center)
    let fade_end_angle = 50.0;    // Outer fade (edge)
    var angle_factor = 1.0;
    
    if (lut_angle < fade_start_angle) {
        // Exponential fade from 25° down to 0° (center fade-in)
        let normalized_angle = lut_angle / fade_start_angle;
        angle_factor = pow(normalized_angle, 8.0);
    } else if (lut_angle > fade_end_angle) {
        // Exponential fade from 80° up to higher angles (edge fade-out)
        // At 80°: factor = 1.0, at 180°: factor ≈ 0.0
        let fade_range = 180.0 - fade_end_angle;
        let normalized_angle = (180.0 - lut_angle) / fade_range;
        angle_factor = pow(clamp(normalized_angle, 0.0, 1.0), 8.0);
    }
    
    color *= angle_factor;

    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));

    // Assume rainbow is at a fixed distance (like atmospheric phenomenon)
    let rainbow_distance = 3000000.0; // Rainbow appears at 3000km distance (planetary scale)

    // Calculate ground intersection
    let ground_t = intersect_plane(params.camera_pos, ray_dir, vec3<f32>(0.0, 1.0, 0.0), 0.0);
    let ground_visible = ground_t > 0.0 && ground_t < 500000.0;
    let ground_occludes_rainbow = ground_visible && ground_t < rainbow_distance;

    // Start with background (stars or ground)
    var bg_color = stars(ray_dir);
    if (ground_visible) {
        let world_hit = params.camera_pos + ray_dir * ground_t;
        bg_color = ground_surface(world_hit);
    }

    // Additive blending: rainbow light is added on top of background
    var final_color: vec3<f32>;
    if (ground_occludes_rainbow) {
        // Ground blocks rainbow - only show ground
        final_color = bg_color;
    } else {
        // Rainbow + dimmed background (30% stars/ground when rainbow visible)
        final_color = bg_color * 0.3 + color;
    }

    return vec4<f32>(final_color, 1.0);
}
