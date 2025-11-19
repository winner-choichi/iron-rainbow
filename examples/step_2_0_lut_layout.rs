//! Step 2.0: LUT Layout Preview
//!
//! Reads `configs/lut_config.toml` and renders a conceptual diagram showing
//! how the wavelength (rows) and exit-angle (columns) axes map to the LUT texture.
//! This is a design-time visualization so that we can confirm parameters before
//! implementing the actual LUT generator.
//!
//! Run with:
//! ```bash
//! cargo run --example step_2_0_lut_layout
//! ```

use image::Rgb;
use iron_rainbow::{
    lut::{FalseColorStop, LutConfig},
    Renderer2D,
};
use std::fs;

fn interpolate_false_color(stops: &[FalseColorStop], wavelength: f32) -> [u8; 3] {
    if stops.is_empty() {
        return [200, 200, 200];
    }
    if wavelength <= stops[0].wavelength {
        return stops[0].color;
    }
    if wavelength >= stops[stops.len() - 1].wavelength {
        return stops[stops.len() - 1].color;
    }
    for pair in stops.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        if wavelength >= a.wavelength && wavelength <= b.wavelength {
            let span = b.wavelength - a.wavelength;
            let t = if span.abs() < f32::EPSILON {
                0.0
            } else {
                (wavelength - a.wavelength) / span
            };
            let mix =
                |c1: u8, c2: u8| -> u8 { (c1 as f32 * (1.0 - t) + c2 as f32 * t).round() as u8 };
            return [
                mix(a.color[0], b.color[0]),
                mix(a.color[1], b.color[1]),
                mix(a.color[2], b.color[2]),
            ];
        }
    }
    stops[stops.len() - 1].color
}

fn apply_intensity(color: [u8; 3], factor: f32) -> [u8; 3] {
    let f = factor.clamp(0.0, 1.0);
    [
        (color[0] as f32 * f).round() as u8,
        (color[1] as f32 * f).round() as u8,
        (color[2] as f32 * f).round() as u8,
    ]
}

fn main() {
    const CONFIG_PATH: &str = "configs/lut_config.toml";
    let config = LutConfig::load(CONFIG_PATH).expect("Failed to load configs/lut_config.toml");

    println!("Step 2.0: LUT Layout Preview");
    println!("============================\n");
    println!("Loaded configuration from {}", CONFIG_PATH);
    println!(
        "Wavelength axis: {:.1}–{:.1} nm ({} rows)",
        config.grid.wavelength_min_nm, config.grid.wavelength_max_nm, config.grid.wavelength_steps
    );
    println!(
        "Angle axis: {:.1}°–{:.1}° ({} columns)",
        config.grid.angle_min_deg, config.grid.angle_max_deg, config.grid.angle_steps
    );
    println!(
        "Angle supersample: {}x",
        config.grid.angle_supersample.unwrap_or(1)
    );
    println!(
        "Impact parameters: {:.2}–{:.2} ({} samples)",
        config.droplet.impact_min, config.droplet.impact_max, config.droplet.impact_samples
    );
    println!(
        "Wavelength step: {:.2} nm (batch size {}), rays/λ: {}",
        config.wavelength.wavelength_step_nm,
        config.wavelength.batch_size,
        config.droplet.rays_per_wavelength
    );
    println!(
        "Output format: {} | texture: {} | metadata: {}",
        config.output.format, config.output.texture_path, config.output.metadata_path
    );
    println!("False-color stops: {:?}\n", config.renderer.false_color);

    // Prepare renderer
    let width = 1920;
    let height = 1080;
    let mut renderer = Renderer2D::new(width, height, 15.0);
    let bg_color = Rgb([246, 249, 255]);
    renderer.fill_rect(-6.0, 6.0, 12.0, 12.0, bg_color);

    // Define LUT area in world coordinates
    let lut_x0 = -5.0;
    let lut_x1 = 3.0;
    let lut_y0 = -4.0;
    let lut_y1 = 4.0;

    // Draw LUT background panel
    renderer.fill_rect(
        lut_x0,
        lut_y1,
        lut_x1 - lut_x0,
        lut_y1 - lut_y0,
        Rgb([230, 236, 247]),
    );
    renderer.draw_thick_line(lut_x0, lut_y0, lut_x1, lut_y0, 0.04, Rgb([90, 110, 150]));
    renderer.draw_thick_line(lut_x0, lut_y1, lut_x1, lut_y1, 0.04, Rgb([90, 110, 150]));
    renderer.draw_thick_line(lut_x0, lut_y0, lut_x0, lut_y1, 0.04, Rgb([90, 110, 150]));
    renderer.draw_thick_line(lut_x1, lut_y0, lut_x1, lut_y1, 0.04, Rgb([90, 110, 150]));

    let angle_range = config.grid.angle_max_deg - config.grid.angle_min_deg;
    let wavelength_range = config.grid.wavelength_max_nm - config.grid.wavelength_min_nm;

    let angle_to_x = |angle: f32| {
        let t = (angle - config.grid.angle_min_deg) / angle_range;
        lut_x0 + t * (lut_x1 - lut_x0)
    };
    let wl_to_y = |wl: f32| {
        let t = (wl - config.grid.wavelength_min_nm) / wavelength_range;
        lut_y1 - t * (lut_y1 - lut_y0)
    };

    // Paint a conceptual LUT heatmap (wavelength color × angle-intensity gradient)
    let lut_width = lut_x1 - lut_x0;
    let lut_height = lut_y1 - lut_y0;
    let row_height = lut_height / config.grid.wavelength_steps as f32;
    let samples_per_row = (config.grid.angle_steps / 32).max(16);
    let sample_width = lut_width / samples_per_row as f32;

    for row in 0..config.grid.wavelength_steps {
        let wl = config.grid.wavelength_min_nm
            + (row as f32 + 0.5) / config.grid.wavelength_steps as f32 * wavelength_range;
        let base_color = interpolate_false_color(&config.renderer.false_color, wl);
        let y_top = lut_y1 - row as f32 * row_height;

        for col in 0..samples_per_row {
            let t = if samples_per_row <= 1 {
                0.0
            } else {
                col as f32 / (samples_per_row as f32 - 1.0)
            };
            let intensity = (1.0 - (t - 0.5).abs() * 1.6).max(0.1);
            let tint = apply_intensity(base_color, intensity);
            let x_left = lut_x0 + col as f32 * sample_width;
            renderer.fill_rect(
                x_left,
                y_top,
                sample_width * 1.05,
                row_height * 0.95,
                Rgb(tint),
            );
        }
    }

    let (lut_px0, lut_py1) = renderer.world_to_pixel_coords(lut_x0, lut_y1);
    let (lut_px1, lut_py0) = renderer.world_to_pixel_coords(lut_x1, lut_y0);

    // Draw major grid lines (every 10° and every 5 nm)
    for (idx, angle) in (config.grid.angle_min_deg as i32..=config.grid.angle_max_deg as i32)
        .step_by(10)
        .enumerate()
    {
        let x = angle_to_x(angle as f32);
        renderer.draw_line(x, lut_y0, x, lut_y1, Rgb([200, 210, 230]));
        let text = format!("{}°", angle);
        let (tick_px, _tick_py) = renderer.world_to_pixel_coords(x, lut_y0);
        // place labels 90px below LUT bottom with staggering
        let base_y = lut_py0 as f32 + 90.0;
        let offset = if idx % 2 == 0 { 0.0 } else { 24.0 };
        renderer.draw_text_screen(
            tick_px as f32 - 18.0,
            base_y + offset,
            &text,
            22.0,
            Rgb([60, 60, 90]),
        );
    }

    for (idx, wl) in (config.grid.wavelength_min_nm as i32..=config.grid.wavelength_max_nm as i32)
        .step_by(5)
        .enumerate()
    {
        let y = wl_to_y(wl as f32);
        renderer.draw_line(lut_x0, y, lut_x1, y, Rgb([200, 210, 230]));
        let text = format!("{} nm", wl);
        let (_tick_px, tick_py) = renderer.world_to_pixel_coords(lut_x1, y);
        // place labels 60px to the right, staggering vertically
        let base_x = lut_px1 as f32 + 80.0;
        let offset = if idx % 2 == 0 { 0.0 } else { 20.0 };
        renderer.draw_text_screen(
            base_x + 20.0,
            tick_py as f32 + offset,
            &text,
            22.0,
            Rgb([60, 60, 90]),
        );
    }

    // Axis labels with dedicated margins (screen space)
    renderer.draw_text_screen(
        lut_px0 as f32 + 100.0,
        lut_py1 as f32 - 80.0,
        "Wavelength axis (145–200 nm)",
        30.0,
        Rgb([30, 40, 90]),
    );
    renderer.draw_text_screen(
        lut_px0 as f32 + 80.0,
        lut_py0 as f32 + 120.0,
        "Exit angle axis (-180° to -120°)",
        30.0,
        Rgb([30, 40, 90]),
    );
    renderer.draw_arrow(
        lut_x0,
        lut_y0 - 0.8,
        lut_x1,
        lut_y0 - 0.8,
        Rgb([60, 70, 110]),
    );
    renderer.draw_arrow(
        lut_x0 - 0.8,
        lut_y0 - 0.2,
        lut_x0 - 0.8,
        lut_y1,
        Rgb([60, 70, 110]),
    );
    renderer.draw_text_screen(
        lut_px1 as f32 - 220.0,
        lut_py0 as f32 - 60.0,
        "Observation angle",
        24.0,
        Rgb([60, 70, 110]),
    );
    renderer.draw_text_screen(
        lut_px0 as f32 - 220.0,
        lut_py1 as f32 + 30.0,
        "Wavelength",
        24.0,
        Rgb([60, 70, 110]),
    );

    // Callouts for representative wavelengths and angles (screen coordinates)
    let callout_wls = [145.0_f32, 170.0, 190.0];
    for (idx, wl) in callout_wls.iter().enumerate() {
        let y = wl_to_y(*wl);
        renderer.draw_thick_line(lut_x0 - 0.2, y, lut_x0, y, 0.04, Rgb([90, 90, 120]));
        let (tick_px, tick_py) = renderer.world_to_pixel_coords(lut_x0, y);
        renderer.draw_text_screen(
            tick_px as f32 - 220.0,
            tick_py as f32 + 10.0 + idx as f32 * 24.0,
            &format!("{} nm", wl),
            22.0,
            Rgb([60, 60, 90]),
        );
    }

    let callout_angles = [-175.0_f32, -150.0, -125.0];
    for (idx, angle) in callout_angles.iter().enumerate() {
        let x = angle_to_x(*angle);
        renderer.draw_thick_line(x, lut_y0, x, lut_y0 - 0.3, 0.04, Rgb([90, 90, 120]));
        let (tick_px, tick_py) = renderer.world_to_pixel_coords(x, lut_y0);
        renderer.draw_text_screen(
            tick_px as f32 - 18.0,
            tick_py as f32 + 70.0 + idx as f32 * 24.0,
            &format!("{}°", angle),
            22.0,
            Rgb([60, 60, 90]),
        );
    }

    // False-color legend on the right
    let legend_x = 3.5;
    let legend_y_top = 4.0;
    let legend_height = 6.0;
    let (legend_px, legend_py) = renderer.world_to_pixel_coords(legend_x, legend_y_top + 0.5);
    renderer.draw_text_screen(
        legend_px as f32 - 40.0,
        legend_py as f32 - 70.0,
        "False-color mapping",
        26.0,
        Rgb([30, 30, 30]),
    );

    if !config.renderer.false_color.is_empty() {
        let total = config.renderer.false_color.len() as f32;
        for (i, stop) in config.renderer.false_color.iter().enumerate() {
            let t0 = i as f32 / total;
            let t1 = (i as f32 + 1.0) / total;
            let y0 = legend_y_top - t0 * legend_height;
            let y1 = legend_y_top - t1 * legend_height;
            renderer.fill_rect(legend_x, y0, 1.5, y0 - y1, Rgb(stop.color));
            let (label_px, label_py) =
                renderer.world_to_pixel_coords(legend_x + 1.6, (y0 + y1) / 2.0);
            renderer.draw_text_screen(
                label_px as f32 + 4.0,
                label_py as f32 + 6.0,
                &format!("{} nm", stop.wavelength),
                22.0,
                Rgb([40, 40, 40]),
            );
        }
        renderer.draw_thick_line(
            legend_x,
            legend_y_top - legend_height,
            legend_x,
            legend_y_top,
            0.03,
            Rgb([20, 20, 20]),
        );
        renderer.draw_thick_line(
            legend_x + 1.5,
            legend_y_top - legend_height,
            legend_x + 1.5,
            legend_y_top,
            0.03,
            Rgb([20, 20, 20]),
        );
    }

    renderer.draw_text_screen(
        120.0,
        height as f32 - 100.0,
        &format!(
            "Normalize: {} / Exposure: {:.1}",
            config.intensity.normalize_mode, config.intensity.exposure
        ),
        26.0,
        Rgb([80, 80, 80]),
    );
    renderer.draw_text_screen(
        120.0,
        height as f32 - 60.0,
        &format!(
            "Droplet radius: {:.0} nm | Rays/λ: {}",
            config.droplet.radius_um * 1000.0,
            config.droplet.rays_per_wavelength
        ),
        26.0,
        Rgb([80, 80, 80]),
    );

    fs::create_dir_all("output").ok();
    let output_path = "output/step_2_0_lut_layout.png";
    renderer
        .save(output_path)
        .expect("Failed to save LUT layout preview");
    println!("\n✓ Saved: {}", output_path);
}
