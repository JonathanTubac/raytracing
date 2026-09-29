use crate::color::Color;
use crate::image_io::load_image;
use crate::ray_intersect::Face;

/// Imagen cargada en memoria para usarla como textura
#[derive(Debug)]
pub struct ImageTexture {
    width: usize,
    height: usize,
    pixels: Vec<Color>,
    /// Opacidad de cada pixel (0 = hueco, 1 = opaco). `None` si la imagen es toda opaca,
    /// o sea que la textura no dice nada sobre transparencia.
    alpha: Option<Vec<f32>>,
}

impl ImageTexture {
    /// Carga una imagen PNG o PPM. La textura vive lo que dura el programa: se filtra a
    /// proposito para poder guardar solo una referencia, y asi `Texture` sigue siendo `Copy`
    /// (y por lo tanto `Material` e `Intersect` tambien).
    pub fn load(path: &str) -> &'static ImageTexture {
        ImageTexture::load_tinted(path, Color::rgb(1.0, 1.0, 1.0))
    }

    /// Carga una imagen y multiplica cada pixel por `tint`. Minecraft guarda el pasto, las
    /// hojas y el agua en gris y les da color segun el bioma; esto hace lo mismo.
    ///
    /// Las texturas animadas de Minecraft (agua, lava) son una tira vertical de cuadros; se
    /// usa solo el primero.
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

    /// Pixel mas cercano, sin suavizar, para que se vean los pixeles como en Minecraft
    fn sample(&self, u: f32, v: f32) -> (Color, Option<f32>) {
        // u da la vuelta (0 y 1 son el mismo borde), v no
        let x = (u.rem_euclid(1.0) * self.width as f32) as usize;
        let y = (v.clamp(0.0, 1.0) * self.height as f32) as usize;
        let i = y.min(self.height - 1) * self.width + x.min(self.width - 1);

        (self.pixels[i], self.alpha.as_ref().map(|alpha| alpha[i]))
    }
}

/// De donde sale el color base de un material. `u` y `v` van de 0 a 1 sobre la superficie.
#[derive(Debug, Clone, Copy)]
pub enum Texture {
    /// Un solo color en toda la superficie
    Solid(Color),
    /// Cuadros alternados de dos colores; `tiles` es cuantos cuadros hay de polo a polo.
    /// Solo lo usan las pruebas.
    #[cfg(test)]
    Checker { a: Color, b: Color, tiles: u32 },
    /// Una imagen envuelta sobre la superficie
    Image(&'static ImageTexture),
    /// Una imagen distinta arriba, a los lados y abajo, como los bloques de Minecraft
    /// (el pasto es verde arriba, tierra abajo y tierra con un borde verde a los lados)
    Block {
        top: &'static ImageTexture,
        side: &'static ImageTexture,
        bottom: &'static ImageTexture,
    },
}

impl Texture {
    /// Color y opacidad en ese punto de la cara. La opacidad es `None` si la textura no
    /// tiene transparencia.
    pub fn sample(&self, u: f32, v: f32, face: Face) -> (Color, Option<f32>) {
        match self {
            Texture::Solid(color) => (*color, None),
            #[cfg(test)]
            Texture::Checker { a, b, tiles } => {
                // u recorre el doble de distancia que v (vuelta completa vs. media vuelta),
                // asi que se usan el doble de columnas para que los cuadros salgan cuadrados
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

    /// Solo el color, en una cara lateral
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

        // Cada cuadro mide 1/8 en u y 1/4 en v
        assert_eq!(textura.color_at(0.05, 0.1).to_hex(), a.to_hex());
        assert_eq!(textura.color_at(0.15, 0.1).to_hex(), b.to_hex());
        assert_eq!(textura.color_at(0.05, 0.35).to_hex(), b.to_hex());
        assert_eq!(textura.color_at(0.15, 0.35).to_hex(), a.to_hex());
    }

    #[test]
    fn la_imagen_devuelve_el_pixel_que_corresponde() {
        // Imagen de 2x2: arriba rojo y verde, abajo azul y blanco
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
        // v = 1 (el borde de abajo) no se sale de la imagen
        assert_eq!(textura.color_at(0.99, 1.0).to_hex(), 0xFFFFFF);
        // u da la vuelta: u = 1 es el mismo borde que u = 0
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

        // El primer pixel es gris (148, 148, 148); tenido queda verde
        assert!(color.g > color.r && color.g > color.b, "{color:?}");
    }

    #[test]
    fn de_una_textura_animada_se_usa_solo_el_primer_cuadro() {
        // El agua es una tira de 16x512 (32 cuadros de 16x16)
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
