use crate::image_io::save_png;

/// Pixeles de la imagen en formato 0xRRGGBB, fila por fila de arriba hacia abajo
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

    /// Acceso directo a los pixeles, para que el render escriba cada fila desde su hilo
    pub fn buffer_mut(&mut self) -> &mut [u32] {
        &mut self.buffer
    }

    /// Mezcla la imagen con el remolino morado de un portal del Nether. `amount` es cuanto
    /// lo tapa (0 = nada, 1 = todo) y `time` hace girar el remolino.
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

            // Bandas en espiral que giran con el tiempo
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
