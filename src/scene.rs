use crate::blocks::Blocks;
use crate::camera::Camera;
use crate::color::Color;
use crate::light::Light;
use crate::math::Vec3;
use crate::ray_intersect::Material;
use crate::skybox::Skybox;
use crate::voxel::VoxelWorld;

/// Bloque de lado 1 con centro en la posicion entera (x, y, z)
fn bloque(x: i32, y: i32, z: i32, material: Material) -> ([i32; 3], Material) {
    ([x, y, z], material)
}

/// Vitrina de prueba con los bloques de Minecraft: un piso de pasto con una poza de agua y
/// una fila de cada bloque encima. Esta frente a la camara inicial, que mira hacia -Z, por
/// eso los objetos tienen z negativo.
pub fn objetos() -> VoxelWorld {
    let b = Blocks::load();
    let mut objects = Vec::new();

    // Poza de agua de 2x2 en el piso, con arena en el fondo
    let es_poza = |x: i32, z: i32| (1..=2).contains(&x) && (-4..=-3).contains(&z);

    // Piso de 9x9 bloques; su cara de arriba queda en y = -1.5
    for x in -4..=4 {
        for z in -9..=-1 {
            if es_poza(x, z) {
                objects.push(bloque(x, -2, z, b.water));
                objects.push(bloque(x, -3, z, b.sand));
            } else {
                objects.push(bloque(x, -2, z, b.grass));
            }
        }
    }

    // Fila del fondo: un bloque de cada material opaco
    let fila = [b.stone, b.cobblestone, b.planks, b.dirt, b.diamond, b.gold, b.obsidian];
    for (i, material) in fila.into_iter().enumerate() {
        objects.push(bloque(i as i32 - 3, -1, -7, material));
    }

    // Arbolito: tronco de dos bloques y una copa de hojas
    objects.push(bloque(-3, -1, -4, b.log));
    objects.push(bloque(-3, 0, -4, b.log));
    for (x, y, z) in [(-3, 1, -4), (-4, 0, -4), (-2, 0, -4), (-3, 0, -5), (-3, 0, -3)] {
        objects.push(bloque(x, y, z, b.leaves));
    }

    // Vidrio al frente y al centro, para ver la fila del fondo doblada a traves de el
    objects.push(bloque(0, -1, -4, b.glass));
    objects.push(bloque(-1, -1, -3, b.glowstone));
    // Linterna marina bajo la copa del arbol, donde el sol no llega
    objects.push(bloque(-2, -1, -5, b.sea_lantern));

    VoxelWorld::new(&objects)
}

// Centro de la escena, alrededor del cual orbita la camara
const CENTRO: Vec3 = Vec3::new(0.0, -1.0, -5.0);

// Posicion de la luz del sol: arriba a la izquierda y un poco al frente
const SOL: Vec3 = Vec3::new(-5.0, 8.0, 2.0);

// Cuanto alumbran los bloques emisivos (por cada unidad de su `emission`) y hasta cuantos
// bloques de distancia
const LUZ_EMISIVA: f32 = 1.4;
const ALCANCE_EMISIVO: f32 = 6.0;

/// La camara orbita alrededor del centro del piso, un poco desde arriba
pub fn camara() -> Camera {
    let mut camera = Camera::new(CENTRO, 9.0);
    camera.orbit(0.0, 0.45);
    camera
}

/// Un sol calido que da la luz principal y las sombras, una luz fria y mas debil desde
/// atras a la derecha (la que rebota del cielo) para que las caras en sombra no queden
/// planas, y una luz dentro de cada bloque emisivo del mundo
pub fn luces(world: &VoxelWorld) -> Vec<Light> {
    let mut lights = vec![
        Light::new(SOL, Color::rgb(1.0, 0.95, 0.85), 0.9),
        Light::new(Vec3::new(6.0, 4.0, -12.0), Color::rgb(0.6, 0.7, 1.0), 0.35),
    ];

    for (center, material) in world.emissive_blocks() {
        // La luz tiene el color promedio de la textura del bloque (calida en la piedra
        // luminosa, fria en la linterna marina), llevado a que su canal mas fuerte valga 1
        let c = material.texture.average_color();
        let color = c * (1.0 / c.r.max(c.g).max(c.b).max(1e-3));
        let intensity = LUZ_EMISIVA * material.emission;
        lights.push(Light::from_block(center, color, intensity, ALCANCE_EMISIVO));
    }
    lights
}

/// Cielo de Minecraft con el sol dibujado justo donde esta la luz del sol, asi las sombras
/// caen en la direccion contraria a donde se ve el sol
pub fn cielo() -> Skybox {
    Skybox::minecraft(SOL - CENTRO)
}
