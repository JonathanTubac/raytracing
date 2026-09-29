//! Bloques de Minecraft: cada uno es un material con su propia textura (la original del
//! juego, en assets/textures) y sus propios parametros de albedo, especular, transparencia
//! y reflectividad.

use crate::color::Color;
use crate::ray_intersect::Material;
use crate::texture::{ImageTexture, Texture};

// Colores con los que Minecraft tine las texturas grises en un bioma de llanura
const TINTE_PASTO: Color = Color::new(145, 189, 89);
const TINTE_HOJAS: Color = Color::new(119, 171, 47);
const TINTE_AGUA: Color = Color::new(63, 118, 228);

fn textura(nombre: &str) -> &'static ImageTexture {
    ImageTexture::load(&ruta(nombre))
}

fn textura_tenida(nombre: &str, tinte: Color) -> &'static ImageTexture {
    ImageTexture::load_tinted(&ruta(nombre), tinte)
}

fn ruta(nombre: &str) -> String {
    format!("{}/assets/textures/{nombre}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Todos los bloques del diorama. Cada textura se carga una sola vez aunque el bloque se
/// repita cientos de veces: los materiales solo guardan una referencia a ella.
#[derive(Clone, Copy)]
pub struct Blocks {
    pub grass: Material,
    pub dirt: Material,
    pub stone: Material,
    pub cobblestone: Material,
    pub planks: Material,
    pub log: Material,
    pub leaves: Material,
    pub glass: Material,
    pub water: Material,
    pub sand: Material,
    pub glowstone: Material,
    pub diamond: Material,
    pub gold: Material,
    pub obsidian: Material,
}

impl Blocks {
    pub fn load() -> Blocks {
        let dirt_texture = textura("dirt");

        Blocks {
            // Pasto: verde arriba, tierra abajo y tierra con borde verde a los lados. Mate.
            grass: Material {
                albedo: [0.95, 0.05],
                specular: 8.0,
                ..Material::new(Texture::Block {
                    top: textura_tenida("grass_block_top", TINTE_PASTO),
                    side: textura("grass_block_side"),
                    bottom: dirt_texture,
                })
            },

            // Tierra: totalmente mate
            dirt: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(dirt_texture))
            },

            // Piedra: mate con un brillo muy leve
            stone: Material {
                albedo: [0.9, 0.1],
                specular: 20.0,
                ..Material::new(Texture::Image(textura("stone")))
            },

            // Piedra labrada: como la piedra pero mas rugosa, casi sin brillo
            cobblestone: Material {
                albedo: [0.95, 0.05],
                specular: 12.0,
                ..Material::new(Texture::Image(textura("cobblestone")))
            },

            // Tablones de roble: madera barnizada, brillo suave
            planks: Material {
                albedo: [0.85, 0.15],
                specular: 25.0,
                ..Material::new(Texture::Image(textura("oak_planks")))
            },

            // Tronco de roble: corteza a los lados y los anillos arriba y abajo
            log: Material {
                albedo: [0.95, 0.05],
                specular: 10.0,
                ..Material::new(Texture::Block {
                    top: textura("oak_log_top"),
                    side: textura("oak_log"),
                    bottom: textura("oak_log_top"),
                })
            },

            // Hojas: opacas, pero la textura tiene huecos por donde se ve y pasa la luz
            leaves: Material {
                albedo: [0.9, 0.1],
                specular: 15.0,
                ..Material::new(Texture::Image(textura_tenida("oak_leaves", TINTE_HOJAS)))
            },

            // Vidrio: marco opaco y centro transparente que dobla la luz (indice 1.5),
            // con un reflejo leve y un brillo fuerte y concentrado
            glass: Material {
                albedo: [0.8, 0.6],
                specular: 150.0,
                transparency: 0.9,
                reflectivity: 0.1,
                refractive_index: 1.5,
                ..Material::new(Texture::Image(textura("glass")))
            },

            // Agua: azul, deja ver el fondo doblado (indice 1.33) y refleja el cielo
            water: Material {
                albedo: [0.35, 0.5],
                specular: 90.0,
                transparency: 0.7,
                reflectivity: 0.2,
                refractive_index: 1.33,
                ..Material::new(Texture::Image(textura_tenida("water_still", TINTE_AGUA)))
            },

            // Arena: mate
            sand: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("sand")))
            },

            // Piedra luminosa: por ahora solo su textura; en la fase de materiales emisivos
            // va a dar luz propia
            glowstone: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("glowstone")))
            },

            // Diamante: pulido, refleja bastante y tiene un brillo chiquito y fuerte
            diamond: Material {
                albedo: [0.7, 0.6],
                specular: 250.0,
                reflectivity: 0.3,
                ..Material::new(Texture::Image(textura("diamond_block")))
            },

            // Oro: metal, refleja y brilla con un brillo mas amplio que el diamante
            gold: Material {
                albedo: [0.7, 0.8],
                specular: 120.0,
                reflectivity: 0.25,
                ..Material::new(Texture::Image(textura("gold_block")))
            },

            // Obsidiana: casi negra y lisa como vidrio volcanico, refleja lo que tiene cerca
            obsidian: Material {
                albedo: [0.8, 0.7],
                specular: 400.0,
                reflectivity: 0.2,
                ..Material::new(Texture::Image(textura("obsidian")))
            },
        }
    }
}
