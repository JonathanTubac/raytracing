use crate::math::Vec3;

use crate::camera::Camera;
use crate::color::Color;
use crate::light::Light;
use crate::cube::Cube;
use crate::ray_intersect::{Material, RayIntersect};
use crate::sphere::Sphere;
use crate::texture::Texture;

// Las texturas de imagen viven en la carpeta assets/ del proyecto
const MADERA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/wood.ppm");
const MARMOL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/marble.ppm");

/// Escena de prueba de cubos. Esta frente a la camara inicial, que mira hacia -Z, por eso
/// los objetos tienen z negativo.
pub fn objetos() -> Vec<Box<dyn RayIntersect>> {
    // Goma roja: mate, casi todo luz difusa y un brillo suave y muy tenue
    let goma = Material {
        albedo: [0.9, 0.1],
        specular: 10.0,
        ..Material::new(Texture::Solid(Color::new(200, 40, 40)))
    };

    // Espejo: casi no tiene luz difusa, un brillo especular chiquito y fuerte,
    // y refleja el 80% de lo que tiene alrededor
    let espejo = Material {
        albedo: [0.1, 0.9],
        specular: 1500.0,
        reflectivity: 0.8,
        ..Material::new(Texture::Solid(Color::new(210, 210, 220)))
    };

    // Vidrio: no tiene color propio, deja pasar el 90% de la luz doblandola con indice
    // de refraccion 1.5, refleja un poco (10%) y tiene un brillo especular
    let vidrio = Material {
        albedo: [0.0, 0.5],
        specular: 125.0,
        transparency: 0.9,
        reflectivity: 0.1,
        refractive_index: 1.5,
        ..Material::new(Texture::Solid(Color::new(255, 255, 255)))
    };

    // Madera: textura de imagen, mate con un poco de brillo
    let madera = Material {
        albedo: [0.85, 0.15],
        specular: 20.0,
        ..Material::new(Texture::from_file(MADERA))
    };

    // Marmol pulido: textura de imagen, brillo marcado y un reflejo leve
    let marmol = Material {
        albedo: [0.7, 0.3],
        specular: 80.0,
        reflectivity: 0.15,
        ..Material::new(Texture::from_file(MARMOL))
    };

    // Ajedrez: textura generada por codigo, sin imagen
    let ajedrez = Material {
        albedo: [0.8, 0.2],
        specular: 40.0,
        ..Material::new(Texture::Checker {
            a: Color::new(240, 240, 240),
            b: Color::new(40, 40, 60),
            tiles: 6,
        })
    };

    let mut objects: Vec<Box<dyn RayIntersect>> = Vec::new();

    // Piso de 7x7 bloques que alterna madera y marmol como un tablero; su cara de arriba
    // queda en y = -1.5
    for i in -3..=3 {
        for k in -3..=3 {
            let material = if (i + k) % 2 == 0 { marmol } else { madera };
            let center = Vec3::new(i as f32, -2.0, -5.0 + k as f32);
            objects.push(Box::new(Cube::new(center, 1.0, material)));
        }
    }

    // Bloques apoyados sobre el piso
    let bloques = [
        // El vidrio va al frente y al centro, asi se ve la goma doblada a traves de el
        (Vec3::new(0.0, -0.9, -3.5), 1.2, vidrio),
        (Vec3::new(0.3, -1.0, -7.0), 1.0, goma),
        (Vec3::new(2.3, -0.75, -5.8), 1.5, espejo),
        (Vec3::new(1.6, -1.2, -3.2), 0.6, ajedrez),
        // Torre de dos bloques de madera
        (Vec3::new(-2.2, -1.0, -5.5), 1.0, madera),
        (Vec3::new(-2.2, 0.0, -5.5), 1.0, madera),
    ];
    for (center, size, material) in bloques {
        objects.push(Box::new(Cube::new(center, size, material)));
    }

    // Esfera de marmol encima de la torre
    objects.push(Box::new(Sphere {
        center: Vec3::new(-2.2, 1.0, -5.5),
        radius: 0.5,
        material: marmol,
    }));

    objects
}

/// La camara orbita alrededor del centro del piso, un poco desde arriba
pub fn camara() -> Camera {
    let mut camera = Camera::new(Vec3::new(0.0, -1.0, -5.0), 8.0);
    camera.orbit(0.0, 0.45);
    camera
}

/// Luz arriba a la izquierda y un poco al frente de la escena
pub fn luz() -> Light {
    Light::new(Vec3::new(-5.0, 6.0, 2.0), 1.0)
}
