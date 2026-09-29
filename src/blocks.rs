//! Bloques de Minecraft: cada uno es un material con su propia textura (la original del
//! juego, en assets/textures) y sus propios parametros de albedo, especular, transparencia
//! y reflectividad.

use crate::color::Color;
use crate::normal_map::NormalMap;
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

/// El mismo material con un mapa normal generado de su propia textura. `fuerza` es cuanto
/// relieve tiene: poco para superficies casi lisas (arena, metal pulido) y mucho para las
/// muy rugosas (piedra labrada).
fn con_relieve(material: Material, fuerza: f32) -> Material {
    Material {
        normal_map: NormalMap::from_texture(&material.texture, fuerza),
        ..material
    }
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
    pub sea_lantern: Material,
    pub diamond: Material,
    pub gold: Material,
    pub obsidian: Material,
    pub bedrock: Material,
    pub deepslate: Material,
    pub gravel: Material,
    pub coal_ore: Material,
    pub iron_ore: Material,
    pub gold_ore: Material,
    pub diamond_ore: Material,
    pub deepslate_gold_ore: Material,
    pub deepslate_diamond_ore: Material,
    pub amethyst: Material,
    pub lava: Material,
    pub nether_portal: Material,
    // Nether
    pub netherrack: Material,
    pub nether_bricks: Material,
    pub soul_sand: Material,
    pub basalt: Material,
    pub blackstone: Material,
    pub crimson_nylium: Material,
    pub crimson_stem: Material,
    pub nether_wart: Material,
    pub shroomlight: Material,
    pub magma: Material,
    pub quartz_ore: Material,
    pub nether_gold_ore: Material,
    pub ancient_debris: Material,
    pub crying_obsidian: Material,
}

impl Blocks {
    pub fn load() -> Blocks {
        let dirt_texture = textura("dirt");

        let b = Blocks {
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

            // Piedra luminosa: emite luz calida propia y alumbra lo que tiene alrededor.
            // Casi no refleja la luz de afuera, porque su propio brillo la tapa.
            glowstone: Material {
                albedo: [0.4, 0.0],
                emission: 0.9,
                ..Material::new(Texture::Image(textura("glowstone")))
            },

            // Linterna marina: emite una luz fria, casi blanca, y es algo pulida
            sea_lantern: Material {
                albedo: [0.4, 0.3],
                specular: 60.0,
                reflectivity: 0.05,
                emission: 0.9,
                ..Material::new(Texture::Image(textura("sea_lantern")))
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

            // Bedrock: la base de la isla. Mate y aspero.
            bedrock: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("bedrock")))
            },

            // Deepslate: la piedra oscura de lo profundo, con vetas distintas arriba y al lado
            deepslate: Material {
                albedo: [0.92, 0.08],
                specular: 18.0,
                ..Material::new(Texture::Block {
                    top: textura("deepslate_top"),
                    side: textura("deepslate"),
                    bottom: textura("deepslate_top"),
                })
            },

            // Grava: el fondo del estanque subterraneo, mate
            gravel: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("gravel")))
            },

            // Minerales: la piedra con vetas. Los metales y el diamante tienen un brillo
            // chiquito y fuerte, como si las vetas destellaran con la luz.
            coal_ore: Material {
                albedo: [0.92, 0.08],
                specular: 20.0,
                ..Material::new(Texture::Image(textura("coal_ore")))
            },
            iron_ore: Material {
                albedo: [0.85, 0.3],
                specular: 90.0,
                ..Material::new(Texture::Image(textura("iron_ore")))
            },
            gold_ore: Material {
                albedo: [0.85, 0.4],
                specular: 120.0,
                ..Material::new(Texture::Image(textura("gold_ore")))
            },
            diamond_ore: Material {
                albedo: [0.85, 0.5],
                specular: 200.0,
                ..Material::new(Texture::Image(textura("diamond_ore")))
            },
            deepslate_gold_ore: Material {
                albedo: [0.85, 0.4],
                specular: 120.0,
                ..Material::new(Texture::Image(textura("deepslate_gold_ore")))
            },
            deepslate_diamond_ore: Material {
                albedo: [0.85, 0.5],
                specular: 200.0,
                ..Material::new(Texture::Image(textura("deepslate_diamond_ore")))
            },

            // Amatista: cristal de una geoda. Muy brillante y refleja un poco lo que la rodea.
            amethyst: Material {
                albedo: [0.75, 0.6],
                specular: 180.0,
                reflectivity: 0.2,
                ..Material::new(Texture::Image(textura("amethyst_block")))
            },

            // Lava: emite una luz naranja fuerte que alumbra la caverna. Casi no refleja la
            // luz de afuera porque su propio brillo la tapa.
            lava: Material {
                albedo: [0.3, 0.15],
                specular: 30.0,
                emission: 1.1,
                ..Material::new(Texture::Image(textura("lava_still")))
            },

            // Portal al Nether: una cortina morada translucida que brilla con luz propia.
            // No dobla la luz (indice 1): se ve el otro lado tenido de morado.
            nether_portal: Material {
                albedo: [0.6, 0.3],
                specular: 60.0,
                transparency: 0.55,
                refractive_index: 1.0,
                emission: 0.8,
                ..Material::new(Texture::Image(textura("nether_portal")))
            },

            // Netherrack: la roca rojiza del Nether, aspera y mate
            netherrack: Material {
                albedo: [0.95, 0.05],
                specular: 10.0,
                ..Material::new(Texture::Image(textura("netherrack")))
            },

            // Ladrillos del Nether: los de las fortalezas, oscuros y con algo de brillo
            nether_bricks: Material {
                albedo: [0.85, 0.2],
                specular: 40.0,
                ..Material::new(Texture::Image(textura("nether_bricks")))
            },

            // Arena de almas: mate, se ven las caras de las almas
            soul_sand: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("soul_sand")))
            },

            // Basalto: columnas de roca volcanica, con vetas verticales a los lados
            basalt: Material {
                albedo: [0.85, 0.2],
                specular: 35.0,
                ..Material::new(Texture::Block {
                    top: textura("basalt_top"),
                    side: textura("basalt_side"),
                    bottom: textura("basalt_top"),
                })
            },

            // Piedra negra: la base de la isla, oscura y algo pulida
            blackstone: Material {
                albedo: [0.85, 0.25],
                specular: 50.0,
                ..Material::new(Texture::Block {
                    top: textura("blackstone_top"),
                    side: textura("blackstone"),
                    bottom: textura("blackstone_top"),
                })
            },

            // Nylium carmesi: el "pasto" del bosque carmesi, sobre netherrack
            crimson_nylium: Material {
                albedo: [0.95, 0.05],
                specular: 8.0,
                ..Material::new(Texture::Block {
                    top: textura("crimson_nylium"),
                    side: textura("crimson_nylium_side"),
                    bottom: textura("netherrack"),
                })
            },

            // Tallo carmesi: el tronco de los hongos gigantes
            crimson_stem: Material {
                albedo: [0.9, 0.1],
                specular: 15.0,
                ..Material::new(Texture::Block {
                    top: textura("crimson_stem_top"),
                    side: textura("crimson_stem"),
                    bottom: textura("crimson_stem_top"),
                })
            },

            // Bloque de verruga: el sombrero de los hongos, carnoso y un poco brillante
            nether_wart: Material {
                albedo: [0.85, 0.2],
                specular: 25.0,
                ..Material::new(Texture::Image(textura("nether_wart_block")))
            },

            // Shroomlight: la luz que cuelga de los hongos carmesi, calida y emisiva
            shroomlight: Material {
                albedo: [0.4, 0.1],
                emission: 1.0,
                ..Material::new(Texture::Image(textura("shroomlight")))
            },

            // Magma: roca con grietas de lava que brillan un poco
            magma: Material {
                albedo: [0.7, 0.2],
                specular: 30.0,
                emission: 0.45,
                ..Material::new(Texture::Image(textura("magma")))
            },

            // Cuarzo y oro del Nether: vetas claras que destellan
            quartz_ore: Material {
                albedo: [0.85, 0.5],
                specular: 150.0,
                ..Material::new(Texture::Image(textura("nether_quartz_ore")))
            },
            nether_gold_ore: Material {
                albedo: [0.85, 0.45],
                specular: 120.0,
                ..Material::new(Texture::Image(textura("nether_gold_ore")))
            },

            // Ancient debris: el mineral mas raro, muy duro, con un brillo metalico apagado
            ancient_debris: Material {
                albedo: [0.8, 0.4],
                specular: 70.0,
                reflectivity: 0.05,
                ..Material::new(Texture::Block {
                    top: textura("ancient_debris_top"),
                    side: textura("ancient_debris_side"),
                    bottom: textura("ancient_debris_top"),
                })
            },

            // Obsidiana llorona: obsidiana con lagrimas moradas que brillan, lisa y reflectiva
            crying_obsidian: Material {
                albedo: [0.7, 0.6],
                specular: 300.0,
                reflectivity: 0.15,
                emission: 0.35,
                ..Material::new(Texture::Image(textura("crying_obsidian")))
            },
        };

        // Relieve de cada bloque. El vidrio queda liso: su textura es casi toda
        // transparente y el relieve solo ensuciaria lo que se ve a traves. La piedra
        // luminosa tambien: da luz propia, y una superficie que brilla no muestra relieve.
        Blocks {
            grass: con_relieve(b.grass, 0.8),
            dirt: con_relieve(b.dirt, 1.5),
            stone: con_relieve(b.stone, 2.5),
            cobblestone: con_relieve(b.cobblestone, 3.5),
            planks: con_relieve(b.planks, 2.0),
            log: con_relieve(b.log, 2.5),
            leaves: con_relieve(b.leaves, 1.0),
            // En el agua el relieve son ondas suaves: dobla distinto la luz que la atraviesa
            // y la que refleja, asi el fondo y el cielo se ven ondulados. Se desenfoca la
            // textura antes de sacar el relieve; con el detalle pixel a pixel el agua se
            // veia como un mosaico de reflejos sueltos.
            water: Material {
                normal_map: NormalMap::smooth_from_texture(&b.water.texture, 3.0, 2),
                ..b.water
            },
            sand: con_relieve(b.sand, 1.2),
            diamond: con_relieve(b.diamond, 2.0),
            gold: con_relieve(b.gold, 2.0),
            obsidian: con_relieve(b.obsidian, 2.0),
            bedrock: con_relieve(b.bedrock, 3.0),
            deepslate: con_relieve(b.deepslate, 2.5),
            gravel: con_relieve(b.gravel, 2.0),
            coal_ore: con_relieve(b.coal_ore, 2.5),
            iron_ore: con_relieve(b.iron_ore, 2.5),
            gold_ore: con_relieve(b.gold_ore, 2.5),
            diamond_ore: con_relieve(b.diamond_ore, 2.5),
            deepslate_gold_ore: con_relieve(b.deepslate_gold_ore, 2.5),
            deepslate_diamond_ore: con_relieve(b.deepslate_diamond_ore, 2.5),
            amethyst: con_relieve(b.amethyst, 1.5),
            netherrack: con_relieve(b.netherrack, 2.5),
            nether_bricks: con_relieve(b.nether_bricks, 3.0),
            soul_sand: con_relieve(b.soul_sand, 2.0),
            basalt: con_relieve(b.basalt, 2.5),
            blackstone: con_relieve(b.blackstone, 2.5),
            crimson_nylium: con_relieve(b.crimson_nylium, 1.2),
            crimson_stem: con_relieve(b.crimson_stem, 2.0),
            nether_wart: con_relieve(b.nether_wart, 1.5),
            quartz_ore: con_relieve(b.quartz_ore, 2.5),
            nether_gold_ore: con_relieve(b.nether_gold_ore, 2.5),
            ancient_debris: con_relieve(b.ancient_debris, 2.5),
            ..b
        }
    }
}
