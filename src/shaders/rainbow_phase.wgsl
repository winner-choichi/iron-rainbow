// Rainbow Phase Function Shader
// Based on "Physically-Based Simulation of Rainbows" (SIGGRAPH 2012)
// Uses pre-computed phase function (LUT) for scattering angle -> intensity mapping

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
    channel_r: f32,
    channel_g: f32,
    channel_b: f32,
    _pad5: f32,
}

@group(0) @binding(0) var lut_texture: texture_2d<f32>;
@group(0) @binding(1) var lut_sampler: sampler;
@group(0) @binding(2) var<uniform> params: ViewerUniform;

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

fn grid_color(world_pos: vec3<f32>) -> vec3<f32> {
    let spacing = 50.0;
    let line_width = 0.6;
    let u = world_pos.x;
    let v = world_pos.z;

    let u_mod = abs(fract(u / spacing) - 0.5) * spacing;
    let v_mod = abs(fract(v / spacing) - 0.5) * spacing;

    var color = vec3<f32>(0.0);

    if (u_mod < line_width || v_mod < line_width) {
        color = vec3<f32>(0.15, 0.15, 0.18);
    }

    if (abs(u) < line_width) {
        color = vec3<f32>(0.0, 0.7, 0.1);
    }
    if (abs(v) < line_width) {
        color = vec3<f32>(0.0, 0.3, 0.8);
    }

    return color;
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

    // Calculate scattering angle θ
    // θ = angle between ray direction and ANTI-SOLAR direction (opposite of sun)
    // Rainbow appears as a cone around the anti-solar point
    let anti_solar = -params.sun_dir;
    let cos_theta = dot(ray_dir, anti_solar);
    let theta_rad = acos(clamp(cos_theta, -1.0, 1.0));
    let theta_deg = theta_rad * RAD_TO_DEG;

    // Debug mode 7: Scattering angle visualization
    if (params.debug_mode == 7u) {
        let norm = theta_deg / 180.0;
        return vec4<f32>(norm, norm, norm, 1.0);
    }

    // Debug mode 6: Show only rainbow range (120° - 180°)
    if (params.debug_mode == 6u) {
        if (theta_deg >= 120.0 && theta_deg <= 180.0) {
            let norm = (theta_deg - 120.0) / 60.0;
            return vec4<f32>(0.0, norm, 1.0 - norm, 1.0);
        } else {
            return vec4<f32>(0.1, 0.1, 0.1, 1.0);
        }
    }

    // Map physical angle (0° - 180°) to LUT angle range
    // LUT stores -180° to -120° which represents 120° to 180° backscattering
    let lut_angle = -180.0 + (theta_deg - 120.0);

    // Debug mode 5: LUT range check
    if (params.debug_mode == 5u) {
        if (lut_angle >= params.angle_min_deg &&
            lut_angle <= params.angle_min_deg + params.angle_range_deg) {
            return vec4<f32>(0.0, 1.0, 0.0, 1.0); // Green = in range
        } else {
            return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red = out of range
        }
    }

    // Sample phase function for RGB channels
    let intensity_r = sample_phase_function(lut_angle, params.channel_r);
    let intensity_g = sample_phase_function(lut_angle, params.channel_g);
    let intensity_b = sample_phase_function(lut_angle, params.channel_b);

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

    // Normal rendering: combine all channels
    var color = vec3<f32>(intensity_r, intensity_g, intensity_b);

    // Apply exposure
    color *= params.exposure;

    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));

    // Background sky
    var bg_color = vec3<f32>(0.25, 0.35, 0.55);

    // World-space ground grid (XZ plane at y=0)
    let ground_t = intersect_plane(params.camera_pos, ray_dir, vec3<f32>(0.0, 1.0, 0.0), 0.0);
    if (ground_t > 0.0) {
        let world_hit = params.camera_pos + ray_dir * ground_t;
        let g_color = grid_color(world_hit);
        if (length(g_color) > 0.0) {
            bg_color = mix(bg_color, g_color, 0.8);
        }
    }

    // Horizon shading based on ray elevation
    let horizon = clamp(ray_dir.y * 0.5 + 0.5, 0.0, 1.0);
    bg_color *= mix(0.6, 1.0, horizon);

    let final_color = color + bg_color * 0.2;
    return vec4<f32>(final_color, 1.0);
}
