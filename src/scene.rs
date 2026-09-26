use nalgebra_glm::Vec3;

use crate::camera::Camera;
use crate::color::Color;
use crate::light::Light;
use crate::ray_intersect::Material;
use crate::sphere::Sphere;
use crate::texture::Texture;

// Las texturas de imagen viven en la carpeta assets/ del proyecto
const MADERA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/wood.png");
const MARMOL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/marble.png");

/// Las esferas estan frente a la camara inicial, que mira hacia -Z, por eso tienen z negativo.
pub fn esferas() -> Vec<Sphere> {
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

    vec![
        // El vidrio va al frente y al centro, asi se ve la roja doblada a traves de el
        Sphere {
            center: Vec3::new(0.0, 0.0, -4.0),
            radius: 1.2,
            material: vidrio,
        },
        Sphere {
            center: Vec3::new(0.3, 0.2, -8.5),
            radius: 1.5,
            material: goma,
        },
        Sphere {
            center: Vec3::new(-3.2, 0.2, -6.0),
            radius: 1.5,
            material: madera,
        },
        Sphere {
            center: Vec3::new(3.4, 0.2, -6.5),
            radius: 1.6,
            material: espejo,
        },
        Sphere {
            center: Vec3::new(1.7, -1.2, -3.6),
            radius: 0.75,
            material: marmol,
        },
        Sphere {
            center: Vec3::new(-1.9, -1.1, -3.8),
            radius: 0.7,
            material: ajedrez,
        },
    ]
}

/// La camara orbita alrededor del centro de las esferas. Empieza en el origen mirando hacia -Z.
pub fn camara() -> Camera {
    Camera::new(Vec3::new(0.0, 0.0, -5.0), 5.0)
}

/// Luz arriba a la izquierda y un poco al frente de las esferas
pub fn luz() -> Light {
    Light::new(Vec3::new(-5.0, 6.0, 2.0), 1.0)
}
