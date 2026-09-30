//! Minecraft blocks: each one with its own texture and parameters

use crate::color::Color;
use crate::normal_map::NormalMap;
use crate::ray_intersect::Material;
use crate::texture::{ImageTexture, Texture};

// Colors Minecraft uses to tint its gray textures in a plains biome
const TINTE_PASTO: Color = Color::new(145, 189, 89);
const TINTE_HOJAS: Color = Color::new(119, 171, 47);
const TINTE_AGUA: Color = Color::new(63, 118, 228);

fn textura(nombre: &str) -> &'static ImageTexture {
    ImageTexture::load(&ruta(nombre))
}

fn textura_tenida(nombre: &str, tinte: Color) -> &'static ImageTexture {
    ImageTexture::load_tinted(&ruta(nombre), tinte)
}

/// The same material with a normal map generated from its own texture
fn con_relieve(material: Material, fuerza: f32) -> Material {
    Material {
        normal_map: NormalMap::from_texture(&material.texture, fuerza),
        ..material
    }
}

fn ruta(nombre: &str) -> String {
    format!("{}/assets/textures/{nombre}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Every block in the diorama
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
            // Grass: green on top, dirt below and dirt with a green edge on the sides. Matte.
            grass: Material {
                albedo: [0.95, 0.05],
                specular: 8.0,
                ..Material::new(Texture::Block {
                    top: textura_tenida("grass_block_top", TINTE_PASTO),
                    side: textura("grass_block_side"),
                    bottom: dirt_texture,
                })
            },

            // Dirt: fully matte
            dirt: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(dirt_texture))
            },

            // Stone: matte with a very faint shine
            stone: Material {
                albedo: [0.9, 0.1],
                specular: 20.0,
                ..Material::new(Texture::Image(textura("stone")))
            },

            // Cobblestone: like stone but rougher, almost no shine
            cobblestone: Material {
                albedo: [0.95, 0.05],
                specular: 12.0,
                ..Material::new(Texture::Image(textura("cobblestone")))
            },

            // Oak planks: varnished wood, soft shine
            planks: Material {
                albedo: [0.85, 0.15],
                specular: 25.0,
                ..Material::new(Texture::Image(textura("oak_planks")))
            },

            // Oak log: bark on the sides and rings on top and bottom
            log: Material {
                albedo: [0.95, 0.05],
                specular: 10.0,
                ..Material::new(Texture::Block {
                    top: textura("oak_log_top"),
                    side: textura("oak_log"),
                    bottom: textura("oak_log_top"),
                })
            },

            // Leaves: opaque, but the texture has holes that you can see and light through
            leaves: Material {
                albedo: [0.9, 0.1],
                specular: 15.0,
                ..Material::new(Texture::Image(textura_tenida("oak_leaves", TINTE_HOJAS)))
            },

            // Glass: opaque frame and a transparent center that bends light (index 1.5)
            glass: Material {
                albedo: [0.8, 0.6],
                specular: 150.0,
                transparency: 0.9,
                reflectivity: 0.1,
                refractive_index: 1.5,
                ..Material::new(Texture::Image(textura("glass")))
            },

            // Water: blue, shows the bottom bent (index 1.33) and reflects the sky
            water: Material {
                albedo: [0.35, 0.5],
                specular: 90.0,
                transparency: 0.7,
                reflectivity: 0.2,
                refractive_index: 1.33,
                ..Material::new(Texture::Image(textura_tenida("water_still", TINTE_AGUA)))
            },

            // Sand: matte
            sand: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("sand")))
            },

            // Glowstone: emits its own warm light and lights up its surroundings
            glowstone: Material {
                albedo: [0.4, 0.0],
                emission: 0.9,
                ..Material::new(Texture::Image(textura("glowstone")))
            },

            // Sea lantern: emits a cold, almost white light, and is somewhat polished
            sea_lantern: Material {
                albedo: [0.4, 0.3],
                specular: 60.0,
                reflectivity: 0.05,
                emission: 0.9,
                ..Material::new(Texture::Image(textura("sea_lantern")))
            },

            // Diamond: polished, quite reflective, with a small and strong highlight
            diamond: Material {
                albedo: [0.7, 0.6],
                specular: 250.0,
                reflectivity: 0.3,
                ..Material::new(Texture::Image(textura("diamond_block")))
            },

            // Gold: metal, reflective, with a wider highlight than diamond
            gold: Material {
                albedo: [0.7, 0.8],
                specular: 120.0,
                reflectivity: 0.25,
                ..Material::new(Texture::Image(textura("gold_block")))
            },

            // Obsidian: almost black and smooth like volcanic glass, reflects what is nearby
            obsidian: Material {
                albedo: [0.8, 0.7],
                specular: 400.0,
                reflectivity: 0.2,
                ..Material::new(Texture::Image(textura("obsidian")))
            },

            // Bedrock: the base of the island. Matte and rough.
            bedrock: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("bedrock")))
            },

            // Deepslate: the dark stone of the depths, with different veins on top and sides
            deepslate: Material {
                albedo: [0.92, 0.08],
                specular: 18.0,
                ..Material::new(Texture::Block {
                    top: textura("deepslate_top"),
                    side: textura("deepslate"),
                    bottom: textura("deepslate_top"),
                })
            },

            // Gravel: the bottom of the underground pool, matte
            gravel: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("gravel")))
            },

            // Ores: stone with veins
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

            // Amethyst: geode crystal. Very shiny and slightly reflective.
            amethyst: Material {
                albedo: [0.75, 0.6],
                specular: 180.0,
                reflectivity: 0.2,
                ..Material::new(Texture::Image(textura("amethyst_block")))
            },

            // Lava: emits a strong orange light that lights up the cave
            lava: Material {
                albedo: [0.3, 0.15],
                specular: 30.0,
                emission: 1.1,
                ..Material::new(Texture::Image(textura("lava_still")))
            },

            // Nether portal: a translucent purple curtain that glows with its own light
            nether_portal: Material {
                albedo: [0.6, 0.3],
                specular: 60.0,
                transparency: 0.55,
                refractive_index: 1.0,
                emission: 0.8,
                ..Material::new(Texture::Image(textura("nether_portal")))
            },

            // Netherrack: the reddish rock of the Nether, rough and matte
            netherrack: Material {
                albedo: [0.95, 0.05],
                specular: 10.0,
                ..Material::new(Texture::Image(textura("netherrack")))
            },

            // Nether bricks: the fortress ones, dark and slightly shiny
            nether_bricks: Material {
                albedo: [0.85, 0.2],
                specular: 40.0,
                ..Material::new(Texture::Image(textura("nether_bricks")))
            },

            // Soul sand: matte, the souls' faces show
            soul_sand: Material {
                albedo: [1.0, 0.0],
                ..Material::new(Texture::Image(textura("soul_sand")))
            },

            // Basalt: volcanic rock columns, with vertical veins on the sides
            basalt: Material {
                albedo: [0.85, 0.2],
                specular: 35.0,
                ..Material::new(Texture::Block {
                    top: textura("basalt_top"),
                    side: textura("basalt_side"),
                    bottom: textura("basalt_top"),
                })
            },

            // Blackstone: the base of the island, dark and somewhat polished
            blackstone: Material {
                albedo: [0.85, 0.25],
                specular: 50.0,
                ..Material::new(Texture::Block {
                    top: textura("blackstone_top"),
                    side: textura("blackstone"),
                    bottom: textura("blackstone_top"),
                })
            },

            // Crimson nylium: the "grass" of the crimson forest, over netherrack
            crimson_nylium: Material {
                albedo: [0.95, 0.05],
                specular: 8.0,
                ..Material::new(Texture::Block {
                    top: textura("crimson_nylium"),
                    side: textura("crimson_nylium_side"),
                    bottom: textura("netherrack"),
                })
            },

            // Crimson stem: the trunk of the huge fungi
            crimson_stem: Material {
                albedo: [0.9, 0.1],
                specular: 15.0,
                ..Material::new(Texture::Block {
                    top: textura("crimson_stem_top"),
                    side: textura("crimson_stem"),
                    bottom: textura("crimson_stem_top"),
                })
            },

            // Nether wart block: the fungus cap, fleshy and slightly shiny
            nether_wart: Material {
                albedo: [0.85, 0.2],
                specular: 25.0,
                ..Material::new(Texture::Image(textura("nether_wart_block")))
            },

            // Shroomlight: the warm, emissive light hanging from crimson fungi
            shroomlight: Material {
                albedo: [0.4, 0.1],
                emission: 1.0,
                ..Material::new(Texture::Image(textura("shroomlight")))
            },

            // Magma: rock with faintly glowing lava cracks
            magma: Material {
                albedo: [0.7, 0.2],
                specular: 30.0,
                emission: 0.45,
                ..Material::new(Texture::Image(textura("magma")))
            },

            // Nether quartz and gold: bright veins that sparkle
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

            // Ancient debris: the rarest ore, very hard, with a dull metallic shine
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

            // Crying obsidian: obsidian with glowing purple tears, smooth and reflective
            crying_obsidian: Material {
                albedo: [0.7, 0.6],
                specular: 300.0,
                reflectivity: 0.15,
                emission: 0.35,
                ..Material::new(Texture::Image(textura("crying_obsidian")))
            },
        };

        // Relief of each block. Glass and water stay flat so the refraction is clear.
        Blocks {
            grass: con_relieve(b.grass, 0.8),
            dirt: con_relieve(b.dirt, 1.5),
            stone: con_relieve(b.stone, 2.5),
            cobblestone: con_relieve(b.cobblestone, 3.5),
            planks: con_relieve(b.planks, 2.0),
            log: con_relieve(b.log, 2.5),
            leaves: con_relieve(b.leaves, 1.0),
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
