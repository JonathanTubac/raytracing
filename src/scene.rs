use crate::blocks::Blocks;
use crate::camera::Camera;
use crate::color::Color;
use crate::cube::Cube;
use crate::light::Light;
use crate::math::Vec3;
use crate::ray_intersect::{Material, RayIntersect};
use crate::skybox::Skybox;

/// Bloque de lado 1 en la posicion entera (x, y, z) de la grilla
fn bloque(x: i32, y: i32, z: i32, material: Material) -> Box<dyn RayIntersect> {
    Box::new(Cube::new(Vec3::new(x as f32, y as f32, z as f32), 1.0, material))
}

/// Vitrina de prueba con los bloques de Minecraft: un piso de pasto con una poza de agua y
/// una fila de cada bloque encima. Esta frente a la camara inicial, que mira hacia -Z, por
/// eso los objetos tienen z negativo.
pub fn objetos() -> Vec<Box<dyn RayIntersect>> {
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

    objects
}

// Centro de la escena, alrededor del cual orbita la camara
const CENTRO: Vec3 = Vec3::new(0.0, -1.0, -5.0);

// Posicion de la luz del sol: arriba a la izquierda y un poco al frente
const SOL: Vec3 = Vec3::new(-5.0, 8.0, 2.0);

/// La camara orbita alrededor del centro del piso, un poco desde arriba
pub fn camara() -> Camera {
    let mut camera = Camera::new(CENTRO, 9.0);
    camera.orbit(0.0, 0.45);
    camera
}

/// Un sol calido que da la luz principal y las sombras, y una luz fria y mas debil desde
/// atras a la derecha (la que rebota del cielo) para que las caras en sombra no queden planas
pub fn luces() -> Vec<Light> {
    vec![
        Light::new(SOL, Color::rgb(1.0, 0.95, 0.85), 0.9),
        Light::new(Vec3::new(6.0, 4.0, -12.0), Color::rgb(0.6, 0.7, 1.0), 0.35),
    ]
}

/// Cielo de Minecraft con el sol dibujado justo donde esta la luz del sol, asi las sombras
/// caen en la direccion contraria a donde se ve el sol
pub fn cielo() -> Skybox {
    Skybox::minecraft(SOL - CENTRO)
}
