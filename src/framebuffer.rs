use crate::image_io::save_png;

/// Image pixels in 0xRRGGBB format, row by row from top to bottom
pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    buffer: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0x000000; width * height],
        }
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        save_png(path, self.width, self.height, &self.buffer)
    }

    pub fn buffer(&self) -> &[u32] {
        &self.buffer
    }

    /// Paints a pixel (0xRRGGBB); does nothing if it is outside the image
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = color;
        }
    }

    /// Pixels as consecutive RGB bytes, the raw format ffmpeg reads
    pub fn to_rgb24(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.buffer.len() * 3);
        for &color in &self.buffer {
            bytes.extend_from_slice(&[(color >> 16) as u8, (color >> 8) as u8, color as u8]);
        }
        bytes
    }

    /// Direct pixel access, so the renderer can write each row from its thread
    pub fn buffer_mut(&mut self) -> &mut [u32] {
        &mut self.buffer
    }

    /// Blends the image with the purple swirl of a Nether portal
    pub fn portal_swirl(&mut self, amount: f32, time: f32) {
        if amount <= 0.0 {
            return;
        }
        let (cx, cy) = (self.width as f32 / 2.0, self.height as f32 / 2.0);
        let scale = 1.0 / cy;

        for (i, pixel) in self.buffer.iter_mut().enumerate() {
            let dx = ((i % self.width) as f32 - cx) * scale;
            let dy = ((i / self.width) as f32 - cy) * scale;
            let radius = (dx * dx + dy * dy).sqrt();

            // Spiral bands that turn over time
            let angle = dy.atan2(dx);
            let s = 0.5 + 0.5 * (angle * 3.0 + radius * 9.0 - time * 5.0).sin();
            let purple = [0.35 + 0.3 * s, 0.05 + 0.12 * s, 0.6 + 0.35 * s];

            let channels = [(*pixel >> 16) & 0xFF, (*pixel >> 8) & 0xFF, *pixel & 0xFF];
            let mixed = [0, 1, 2].map(|c| {
                let original = channels[c] as f32 / 255.0;
                let value = original + (purple[c] - original) * amount;
                (value.clamp(0.0, 1.0) * 255.0).round() as u32
            });
            *pixel = (mixed[0] << 16) | (mixed[1] << 8) | mixed[2];
        }
    }
}
