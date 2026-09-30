use crate::color::Color;
use crate::image_io::load_image;
use crate::ray_intersect::Face;

/// Image loaded in memory to use as a texture
#[derive(Debug, PartialEq)]
pub struct ImageTexture {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
    /// Opacity of each pixel (0 = hole, 1 = opaque)
    alpha: Option<Vec<f32>>,
}

impl ImageTexture {
    /// Loads a PNG or PPM image
    pub fn load(path: &str) -> &'static ImageTexture {
        ImageTexture::load_tinted(path, Color::rgb(1.0, 1.0, 1.0))
    }

    /// Loads an image and multiplies each pixel by `tint`
    pub fn load_tinted(path: &str, tint: Color) -> &'static ImageTexture {
        let image =
            load_image(path).unwrap_or_else(|e| panic!("no se pudo cargar la textura {path}: {e}"));

        let height = image.height.min(image.width);
        let frame = &image.pixels[..image.width * height];

        let pixels = frame
            .iter()
            .map(|p| Color::new(p[0], p[1], p[2]) * tint)
            .collect();
        let has_alpha = frame.iter().any(|p| p[3] < 255);
        let alpha = has_alpha.then(|| frame.iter().map(|p| p[3] as f32 / 255.0).collect());

        Box::leak(Box::new(ImageTexture {
            width: image.width,
            height,
            pixels,
            alpha,
        }))
    }

    /// Handmade texture with opaque pixels, for tests
    #[cfg(test)]
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<Color>) -> &'static ImageTexture {
        Box::leak(Box::new(ImageTexture {
            width,
            height,
            pixels,
            alpha: None,
        }))
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Average color of the visible pixels
    fn average(&self) -> Color {
        let mut sum = Color::black();
        let mut weight = 0.0;
        for (i, pixel) in self.pixels.iter().enumerate() {
            let alpha = self.alpha.as_ref().map_or(1.0, |alpha| alpha[i]);
            sum += *pixel * alpha;
            weight += alpha;
        }
        if weight > 0.0 { sum * (1.0 / weight) } else { Color::black() }
    }

    /// Brightness of pixel (x, y), from 0 to 1
    pub fn luminance(&self, x: isize, y: isize) -> f32 {
        let x = x.rem_euclid(self.width as isize) as usize;
        let y = y.rem_euclid(self.height as isize) as usize;
        let c = self.pixels[y * self.width + x];
        // Human eye weights: green looks brighter than red and blue
        0.299 * c.r + 0.587 * c.g + 0.114 * c.b
    }

    /// Nearest pixel, without smoothing, so pixels show like in Minecraft
    pub fn sample(&self, u: f32, v: f32) -> (Color, Option<f32>) {
        // u wraps around (0 and 1 are the same edge), v doesn't
        let x = (u.rem_euclid(1.0) * self.width as f32) as usize;
        let y = (v.clamp(0.0, 1.0) * self.height as f32) as usize;
        let i = y.min(self.height - 1) * self.width + x.min(self.width - 1);

        (self.pixels[i], self.alpha.as_ref().map(|alpha| alpha[i]))
    }
}

/// Where a material's base color comes from. `u` and `v` go from 0 to 1 over the surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Texture {
    /// A single color over the whole surface
    Solid(Color),
    /// Alternating squares of two colors; `tiles` is how many squares from pole to pole
    #[cfg(test)]
    Checker { a: Color, b: Color, tiles: u32 },
    /// An image wrapped over the surface
    Image(&'static ImageTexture),
    /// One image on top, another on the sides and another at the bottom
    Block {
        top: &'static ImageTexture,
        side: &'static ImageTexture,
        bottom: &'static ImageTexture,
    },
}

impl Texture {
    /// Color and opacity at that point of the face
    pub fn sample(&self, u: f32, v: f32, face: Face) -> (Color, Option<f32>) {
        match self {
            Texture::Solid(color) => (*color, None),
            #[cfg(test)]
            Texture::Checker { a, b, tiles } => {
                // u goes all the way around and v half way: twice the columns for square tiles
                let column = (u * *tiles as f32 * 2.0).floor() as i32;
                let row = (v * *tiles as f32).floor() as i32;

                (if (column + row).rem_euclid(2) == 0 { *a } else { *b }, None)
            }
            Texture::Image(image) => image.sample(u, v),
            Texture::Block { top, side, bottom } => match face {
                Face::Top => top.sample(u, v),
                Face::Side => side.sample(u, v),
                Face::Bottom => bottom.sample(u, v),
            },
        }
    }

    /// Average texture color (the side one, for a block with different faces)
    pub fn average_color(&self) -> Color {
        match self {
            Texture::Solid(color) => *color,
            #[cfg(test)]
            Texture::Checker { a, b, .. } => Color::lerp(*a, *b, 0.5),
            Texture::Image(image) => image.average(),
            Texture::Block { side, .. } => side.average(),
        }
    }

    /// Just the color, on a side face
    #[cfg(test)]
    pub fn color_at(&self, u: f32, v: f32) -> Color {
        self.sample(u, v, Face::Side).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imagen(pixels: Vec<Color>, alpha: Option<Vec<f32>>) -> &'static ImageTexture {
        let lado = (pixels.len() as f32).sqrt() as usize;
        Box::leak(Box::new(ImageTexture {
            width: lado,
            height: lado,
            pixels,
            alpha,
        }))
    }

    #[test]
    fn la_textura_solida_es_igual_en_todos_lados() {
        let textura = Texture::Solid(Color::new(10, 20, 30));

        assert_eq!(textura.color_at(0.0, 0.0).to_hex(), Color::new(10, 20, 30).to_hex());
        assert_eq!(textura.color_at(0.7, 0.3).to_hex(), Color::new(10, 20, 30).to_hex());
    }

    #[test]
    fn el_ajedrez_alterna_los_colores_entre_cuadros_vecinos() {
        let a = Color::new(255, 255, 255);
        let b = Color::new(0, 0, 0);
        let textura = Texture::Checker { a, b, tiles: 4 };

        // Each square is 1/8 in u and 1/4 in v
        assert_eq!(textura.color_at(0.05, 0.1).to_hex(), a.to_hex());
        assert_eq!(textura.color_at(0.15, 0.1).to_hex(), b.to_hex());
        assert_eq!(textura.color_at(0.05, 0.35).to_hex(), b.to_hex());
        assert_eq!(textura.color_at(0.15, 0.35).to_hex(), a.to_hex());
    }

    #[test]
    fn la_imagen_devuelve_el_pixel_que_corresponde() {
        // 2x2 image: red and green on top, blue and white at the bottom
        let textura = Texture::Image(imagen(
            vec![
                Color::new(255, 0, 0),
                Color::new(0, 255, 0),
                Color::new(0, 0, 255),
                Color::new(255, 255, 255),
            ],
            None,
        ));

        assert_eq!(textura.color_at(0.25, 0.25).to_hex(), 0xFF0000);
        assert_eq!(textura.color_at(0.75, 0.25).to_hex(), 0x00FF00);
        assert_eq!(textura.color_at(0.25, 0.75).to_hex(), 0x0000FF);
        // v = 1 (the bottom edge) stays inside the image
        assert_eq!(textura.color_at(0.99, 1.0).to_hex(), 0xFFFFFF);
        // u wraps around: u = 1 is the same edge as u = 0
        assert_eq!(textura.color_at(1.0, 0.25).to_hex(), 0xFF0000);
    }

    #[test]
    fn el_bloque_usa_una_imagen_distinta_arriba_al_lado_y_abajo() {
        let color = |c: Color| imagen(vec![c], None);
        let textura = Texture::Block {
            top: color(Color::new(0, 255, 0)),
            side: color(Color::new(255, 0, 0)),
            bottom: color(Color::new(0, 0, 255)),
        };

        assert_eq!(textura.sample(0.5, 0.5, Face::Top).0.to_hex(), 0x00FF00);
        assert_eq!(textura.sample(0.5, 0.5, Face::Side).0.to_hex(), 0xFF0000);
        assert_eq!(textura.sample(0.5, 0.5, Face::Bottom).0.to_hex(), 0x0000FF);
    }

    #[test]
    fn la_opacidad_solo_existe_si_la_imagen_la_trae() {
        let opaca = imagen(vec![Color::black()], None);
        let con_hueco = imagen(vec![Color::black()], Some(vec![0.0]));

        assert_eq!(opaca.sample(0.5, 0.5).1, None);
        assert_eq!(con_hueco.sample(0.5, 0.5).1, Some(0.0));
    }

    fn textura_de_minecraft(nombre: &str) -> String {
        format!("{}/assets/textures/{nombre}.png", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn el_pasto_gris_se_tine_de_verde() {
        let pasto = ImageTexture::load_tinted(
            &textura_de_minecraft("grass_block_top"),
            Color::new(145, 189, 89),
        );
        let (color, _) = pasto.sample(0.0, 0.0);

        // The first pixel is gray (148, 148, 148); tinted it becomes green
        assert!(color.g > color.r && color.g > color.b, "{color:?}");
    }

    #[test]
    fn de_una_textura_animada_se_usa_solo_el_primer_cuadro() {
        // Water is a 16x512 strip (32 frames of 16x16)
        let agua = ImageTexture::load(&textura_de_minecraft("water_still"));
        assert_eq!((agua.width, agua.height), (16, 16));
        assert_eq!(agua.sample(0.0, 0.0).1, Some(180.0 / 255.0));
    }

    #[test]
    fn el_vidrio_tiene_marco_opaco_y_centro_transparente() {
        let vidrio = ImageTexture::load(&textura_de_minecraft("glass"));

        assert_eq!(vidrio.sample(0.01, 0.01).1, Some(1.0));
        assert_eq!(vidrio.sample(0.5, 0.5).1, Some(0.0));
    }
}
