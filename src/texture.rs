use crate::color::Color;

/// Imagen cargada en memoria para usarla como textura
#[derive(Debug)]
pub struct ImageTexture {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
}

impl ImageTexture {
    fn sample(&self, u: f32, v: f32) -> Color {
        // u da la vuelta (0 y 1 son el mismo borde), v no
        let x = (u.rem_euclid(1.0) * self.width as f32) as usize;
        let y = (v.clamp(0.0, 1.0) * self.height as f32) as usize;

        self.pixels[y.min(self.height - 1) * self.width + x.min(self.width - 1)]
    }
}

/// De donde sale el color base de un material. `u` y `v` van de 0 a 1 sobre la superficie.
#[derive(Debug, Clone, Copy)]
pub enum Texture {
    /// Un solo color en toda la superficie
    Solid(Color),
    /// Cuadros alternados de dos colores; `tiles` es cuantos cuadros hay de polo a polo
    Checker { a: Color, b: Color, tiles: u32 },
    /// Una imagen envuelta sobre la superficie
    Image(&'static ImageTexture),
}

impl Texture {
    /// Carga una imagen (PNG, JPG...) como textura.
    pub fn from_file(path: &str) -> Texture {
        let img = image::open(path)
            .unwrap_or_else(|e| panic!("no se pudo cargar la textura {path}: {e}"))
            .to_rgb8();

        let (width, height) = img.dimensions();
        let pixels = img
            .pixels()
            .map(|p| Color::new(p[0], p[1], p[2]))
            .collect();

        // Se filtra a proposito: la textura vive lo que dura el programa, y asi `Texture`
        // sigue siendo `Copy` (y por lo tanto `Material` e `Intersect` tambien)
        Texture::Image(Box::leak(Box::new(ImageTexture {
            width: width as usize,
            height: height as usize,
            pixels,
        })))
    }

    pub fn color_at(&self, u: f32, v: f32) -> Color {
        match self {
            Texture::Solid(color) => *color,
            Texture::Checker { a, b, tiles } => {
                // u recorre el doble de distancia que v (vuelta completa vs. media vuelta),
                // asi que se usan el doble de columnas para que los cuadros salgan cuadrados
                let column = (u * *tiles as f32 * 2.0).floor() as i32;
                let row = (v * *tiles as f32).floor() as i32;

                if (column + row).rem_euclid(2) == 0 { *a } else { *b }
            }
            Texture::Image(image) => image.sample(u, v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // Cada cuadro mide 1/8 en u y 1/4 en v
        assert_eq!(textura.color_at(0.05, 0.1).to_hex(), a.to_hex());
        assert_eq!(textura.color_at(0.15, 0.1).to_hex(), b.to_hex());
        assert_eq!(textura.color_at(0.05, 0.35).to_hex(), b.to_hex());
        assert_eq!(textura.color_at(0.15, 0.35).to_hex(), a.to_hex());
    }

    #[test]
    fn la_imagen_devuelve_el_pixel_que_corresponde() {
        // Imagen de 2x2: arriba rojo y verde, abajo azul y blanco
        let imagen = ImageTexture {
            width: 2,
            height: 2,
            pixels: vec![
                Color::new(255, 0, 0),
                Color::new(0, 255, 0),
                Color::new(0, 0, 255),
                Color::new(255, 255, 255),
            ],
        };
        let textura = Texture::Image(Box::leak(Box::new(imagen)));

        assert_eq!(textura.color_at(0.25, 0.25).to_hex(), 0xFF0000);
        assert_eq!(textura.color_at(0.75, 0.25).to_hex(), 0x00FF00);
        assert_eq!(textura.color_at(0.25, 0.75).to_hex(), 0x0000FF);
        // v = 1 (el borde de abajo) no se sale de la imagen
        assert_eq!(textura.color_at(0.99, 1.0).to_hex(), 0xFFFFFF);
        // u da la vuelta: u = 1 es el mismo borde que u = 0
        assert_eq!(textura.color_at(1.0, 0.25).to_hex(), 0xFF0000);
    }
}
