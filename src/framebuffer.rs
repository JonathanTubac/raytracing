pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    buffer: Vec<u32>,
    current_color: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            buffer: vec![0x000000; width * height],
            current_color: 0xFFFFFF,
        }
    }

    pub fn set_current_color(&mut self, color: u32) {
        self.current_color = color;
    }

    pub fn point(&mut self, x: usize, y: usize) {
        if x < self.width && y < self.height {
            self.buffer[y * self.width + x] = self.current_color;
        }
    }

    pub fn save_to_file(&self, path: &str) -> image::ImageResult<()> {
        let mut img = image::RgbImage::new(self.width as u32, self.height as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.buffer[y * self.width + x];
                let r = ((color >> 16) & 0xFF) as u8;
                let g = ((color >> 8) & 0xFF) as u8;
                let b = (color & 0xFF) as u8;
                img.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
            }
        }
        img.save(path)
    }
}
