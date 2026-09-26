use nalgebra_glm::Vec3;

use crate::color::Color;
use crate::ray_intersect::Material;
use crate::sphere::Sphere;

const ROJO: Material = Material { diffuse: Color::new(200, 40, 40) };
const VERDE: Material = Material { diffuse: Color::new(60, 170, 80) };
const AZUL: Material = Material { diffuse: Color::new(50, 90, 200) };
const AMARILLO: Material = Material { diffuse: Color::new(230, 200, 60) };

/// La camara esta en el origen mirando hacia -Z, asi que las esferas van con z negativo.
pub fn esferas() -> Vec<Sphere> {
    vec![
        Sphere {
            center: Vec3::new(-1.5, 0.0, -5.0),
            radius: 1.2,
            material: ROJO,
        },
        // Esta esta mas cerca de la camara y tapa parte de la roja
        Sphere {
            center: Vec3::new(-0.3, 0.2, -3.5),
            radius: 0.8,
            material: VERDE,
        },
        Sphere {
            center: Vec3::new(2.5, -0.5, -6.0),
            radius: 1.5,
            material: AZUL,
        },
        Sphere {
            center: Vec3::new(-3.5, 1.5, -7.0),
            radius: 1.5,
            material: AMARILLO,
        },
    ]
}
