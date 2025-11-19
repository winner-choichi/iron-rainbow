/// 2D rendering utilities for visualization
/// Converts simulation data to images
use image::{ImageBuffer, Rgb, RgbImage};

pub struct Renderer2D {
    width: u32,
    height: u32,
    image: RgbImage,
    // World space to pixel space transformation
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl Renderer2D {
    /// Create a new renderer with given dimensions
    /// world_size: size of world coordinate system (e.g., 10.0 means -5 to +5)
    pub fn new(width: u32, height: u32, world_size: f32) -> Self {
        let image = ImageBuffer::from_pixel(width, height, Rgb([255, 255, 255])); // White background
        let scale = width.min(height) as f32 / world_size;
        let offset_x = width as f32 / 2.0;
        let offset_y = height as f32 / 2.0;

        Self {
            width,
            height,
            image,
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Convert world coordinates to pixel coordinates
    fn world_to_pixel(&self, x: f32, y: f32) -> (i32, i32) {
        let px = (x * self.scale + self.offset_x) as i32;
        let py = ((-y) * self.scale + self.offset_y) as i32; // Flip Y axis
        (px, py)
    }

    /// Convert pixel coordinates to world coordinates (top-left origin)
    fn pixel_to_world(&self, px: f32, py: f32) -> (f32, f32) {
        let x = (px - self.offset_x) / self.scale;
        let y = -((py - self.offset_y) / self.scale);
        (x, y)
    }

    /// Return renderer dimensions in pixels
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Public helper to convert world coordinates to pixel coordinates
    pub fn world_to_pixel_coords(&self, x: f32, y: f32) -> (i32, i32) {
        self.world_to_pixel(x, y)
    }

    /// Check if pixel is within bounds
    fn in_bounds(&self, px: i32, py: i32) -> bool {
        px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32
    }

    /// Draw a pixel at world coordinates
    pub fn draw_pixel(&mut self, x: f32, y: f32, color: Rgb<u8>) {
        let (px, py) = self.world_to_pixel(x, y);
        if self.in_bounds(px, py) {
            self.image.put_pixel(px as u32, py as u32, color);
        }
    }

    /// Draw a line using Bresenham's algorithm
    pub fn draw_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgb<u8>) {
        let (mut px0, mut py0) = self.world_to_pixel(x0, y0);
        let (px1, py1) = self.world_to_pixel(x1, y1);

        let dx = (px1 - px0).abs();
        let dy = (py1 - py0).abs();
        let sx = if px0 < px1 { 1 } else { -1 };
        let sy = if py0 < py1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            if self.in_bounds(px0, py0) {
                self.image.put_pixel(px0 as u32, py0 as u32, color);
            }

            if px0 == px1 && py0 == py1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                px0 += sx;
            }
            if e2 < dx {
                err += dx;
                py0 += sy;
            }
        }
    }

    /// Draw a circle (outline)
    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Rgb<u8>) {
        let num_points = 360;
        for i in 0..num_points {
            let angle = (i as f32) * 2.0 * std::f32::consts::PI / (num_points as f32);
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            self.draw_pixel(x, y, color);
        }
    }

    /// Draw a filled circle
    pub fn draw_filled_circle(&mut self, cx: f32, cy: f32, radius: f32, color: Rgb<u8>) {
        let (pcx, pcy) = self.world_to_pixel(cx, cy);
        let pradius = (radius * self.scale) as i32;

        for dy in -pradius..=pradius {
            for dx in -pradius..=pradius {
                if dx * dx + dy * dy <= pradius * pradius {
                    let px = pcx + dx;
                    let py = pcy + dy;
                    if self.in_bounds(px, py) {
                        self.image.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
    }

    /// Draw an arrow (for vectors)
    pub fn draw_arrow(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Rgb<u8>) {
        // Draw main line
        self.draw_line(x0, y0, x1, y1, color);

        // Draw arrowhead
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            let ux = dx / len;
            let uy = dy / len;

            let arrow_len = 0.2_f32;
            let arrow_angle = 0.5_f32;

            // Left wing
            let lx = x1 - arrow_len * (ux * arrow_angle.cos() + uy * arrow_angle.sin());
            let ly = y1 - arrow_len * (uy * arrow_angle.cos() - ux * arrow_angle.sin());
            self.draw_line(x1, y1, lx, ly, color);

            // Right wing
            let rx = x1 - arrow_len * (ux * arrow_angle.cos() - uy * arrow_angle.sin());
            let ry = y1 - arrow_len * (uy * arrow_angle.cos() + ux * arrow_angle.sin());
            self.draw_line(x1, y1, rx, ry, color);
        }
    }

    /// Draw a point (small filled circle)
    pub fn draw_point(&mut self, x: f32, y: f32, color: Rgb<u8>) {
        self.draw_filled_circle(x, y, 0.05, color);
    }

    /// Draw grid for reference
    pub fn draw_grid(&mut self, spacing: f32, color: Rgb<u8>) {
        let world_half = self.width.max(self.height) as f32 / (2.0 * self.scale);

        // Vertical lines
        let mut x = 0.0;
        while x <= world_half {
            self.draw_line(x, -world_half, x, world_half, color);
            if x > 0.0 {
                self.draw_line(-x, -world_half, -x, world_half, color);
            }
            x += spacing;
        }

        // Horizontal lines
        let mut y = 0.0;
        while y <= world_half {
            self.draw_line(-world_half, y, world_half, y, color);
            if y > 0.0 {
                self.draw_line(-world_half, -y, world_half, -y, color);
            }
            y += spacing;
        }
    }

    /// Draw coordinate axes
    pub fn draw_axes(&mut self) {
        let world_half = self.width.max(self.height) as f32 / (2.0 * self.scale);

        // X axis (red)
        self.draw_arrow(-world_half, 0.0, world_half, 0.0, Rgb([200, 0, 0]));

        // Y axis (green)
        self.draw_arrow(0.0, -world_half, 0.0, world_half, Rgb([0, 200, 0]));
    }

    /// Save image to file
    pub fn save(&self, path: &str) -> Result<(), image::ImageError> {
        self.image.save(path)
    }

    /// Get reference to image for further processing
    pub fn image(&self) -> &RgbImage {
        &self.image
    }

    /// Draw an arc (portion of circle) for angle visualization
    pub fn draw_arc(
        &mut self,
        cx: f32,
        cy: f32,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        color: Rgb<u8>,
    ) {
        let num_segments = 50;
        let angle_range = end_angle - start_angle;

        for i in 0..num_segments {
            let t1 = (i as f32) / (num_segments as f32);
            let t2 = ((i + 1) as f32) / (num_segments as f32);

            let a1 = start_angle + t1 * angle_range;
            let a2 = start_angle + t2 * angle_range;

            let x1 = cx + radius * a1.cos();
            let y1 = cy + radius * a1.sin();
            let x2 = cx + radius * a2.cos();
            let y2 = cy + radius * a2.sin();

            self.draw_line(x1, y1, x2, y2, color);
        }
    }

    /// Draw a thick line by drawing multiple parallel lines
    pub fn draw_thick_line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        thickness: f32,
        color: Rgb<u8>,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();

        if len == 0.0 {
            return;
        }

        // Perpendicular direction
        let px = -dy / len;
        let py = dx / len;

        // Draw multiple lines (convert thickness to pixel space)
        let steps = ((thickness * self.scale).max(1.0) as i32).max(3);
        for i in 0..steps {
            let offset = (i as f32 - (steps as f32 / 2.0)) / self.scale;
            self.draw_line(
                x0 + px * offset,
                y0 + py * offset,
                x1 + px * offset,
                y1 + py * offset,
                color,
            );
        }
    }

    /// Draw a dashed line
    pub fn draw_dashed_line(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        dash_length: f32,
        color: Rgb<u8>,
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();

        if len == 0.0 {
            return;
        }

        let ux = dx / len;
        let uy = dy / len;

        let mut t = 0.0;
        let mut drawing = true;

        while t < len {
            let next_t = (t + dash_length).min(len);

            if drawing {
                self.draw_line(
                    x0 + ux * t,
                    y0 + uy * t,
                    x0 + ux * next_t,
                    y0 + uy * next_t,
                    color,
                );
            }

            t = next_t;
            drawing = !drawing;
        }
    }

    /// Fill a rectangular region with color
    pub fn fill_rect(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgb<u8>) {
        let (px1, py1) = self.world_to_pixel(x, y);
        let (px2, py2) = self.world_to_pixel(x + width, y - height);

        let x_min = px1.min(px2).max(0);
        let x_max = px1.max(px2).min(self.width as i32 - 1);
        let y_min = py1.min(py2).max(0);
        let y_max = py1.max(py2).min(self.height as i32 - 1);

        for py in y_min..=y_max {
            for px in x_min..=x_max {
                self.image.put_pixel(px as u32, py as u32, color);
            }
        }
    }

    /// Draw text using a simple bitmap font
    pub fn draw_text(&mut self, x: f32, y: f32, text: &str, size: f32, color: Rgb<u8>) {
        let mut cursor_x = x;
        for ch in text.chars() {
            self.draw_char(cursor_x, y, ch, size, color);
            cursor_x += size * 0.7; // Character spacing
        }
    }

    /// Draw text positioned directly in screen (pixel) coordinates
    pub fn draw_text_screen(&mut self, px: f32, py: f32, text: &str, size_px: f32, color: Rgb<u8>) {
        let (x_world, y_world) = self.pixel_to_world(px, py);
        let size_world = size_px / self.scale;
        self.draw_text(x_world, y_world, text, size_world.max(0.01), color);
    }

    /// Draw a single character using bitmap font
    fn draw_char(&mut self, x: f32, y: f32, ch: char, size: f32, color: Rgb<u8>) {
        let pixel_size = size / 7.0;

        // 5x7 bitmap font patterns (1 = filled, 0 = empty)
        let pattern = match ch {
            '0' => vec![
                0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
            ],
            '1' => vec![
                0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
            '2' => vec![
                0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
            ],
            '3' => vec![
                0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
            ],
            '4' => vec![
                0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
            ],
            '5' => vec![
                0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
            ],
            '6' => vec![
                0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
            ],
            '7' => vec![
                0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
            ],
            '8' => vec![
                0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
            ],
            '9' => vec![
                0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
            ],
            '.' => vec![
                0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
            ],
            '-' => vec![
                0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
            ],
            '+' => vec![
                0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
            ],
            '°' => vec![
                0b01100, 0b10010, 0b10010, 0b01100, 0b00000, 0b00000, 0b00000,
            ],
            'λ' | 'l' => vec![
                0b10000, 0b01000, 0b00100, 0b00100, 0b01010, 0b01010, 0b10001,
            ],
            'n' | 'N' => vec![
                0b10001, 0b11001, 0b10101, 0b10101, 0b10011, 0b10001, 0b10001,
            ],
            'k' | 'K' => vec![
                0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
            ],
            'T' | 't' => vec![
                0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
            ],
            '%' => vec![
                0b11001, 0b11010, 0b00010, 0b00100, 0b01000, 0b01011, 0b10011,
            ],
            '(' => vec![
                0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
            ],
            ')' => vec![
                0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
            ],
            ':' => vec![
                0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000,
            ],
            'A' | 'a' => vec![
                0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
            'B' | 'b' => vec![
                0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
            ],
            'C' | 'c' => vec![
                0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
            ],
            'D' | 'd' => vec![
                0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
            ],
            'E' | 'e' => vec![
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
            ],
            'F' | 'f' => vec![
                0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
            'G' | 'g' => vec![
                0b01110, 0b10001, 0b10000, 0b10011, 0b10001, 0b10001, 0b01110,
            ],
            'H' | 'h' => vec![
                0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
            ],
            'I' | 'i' => vec![
                0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
            ],
            'L' => vec![
                0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
            ],
            'M' | 'm' => vec![
                0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
            ],
            'O' | 'o' => vec![
                0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
            'P' | 'p' => vec![
                0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
            ],
            'R' | 'r' => vec![
                0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
            ],
            'S' | 's' => vec![
                0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
            ],
            'U' | 'u' => vec![
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
            ],
            'V' | 'v' => vec![
                0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
            ],
            'W' | 'w' => vec![
                0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
            ],
            'X' | 'x' => vec![
                0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
            ],
            ' ' => vec![0; 7],
            _ => vec![
                0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111,
            ], // Default: box
        };

        // Draw pixels
        for (row, bits) in pattern.iter().enumerate() {
            for col in 0..5 {
                if (bits >> (4 - col)) & 1 == 1 {
                    let px = x + col as f32 * pixel_size;
                    let py = y - row as f32 * pixel_size;
                    self.fill_rect(px, py, pixel_size * 0.9, pixel_size * 0.9, color);
                }
            }
        }
    }

    /// Draw text label (deprecated, use draw_text instead)
    pub fn draw_label(&mut self, x: f32, y: f32, text: &str, color: Rgb<u8>) {
        self.draw_text(x, y, text, 0.3, color);
    }
}
