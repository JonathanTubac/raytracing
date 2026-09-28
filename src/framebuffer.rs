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
}
