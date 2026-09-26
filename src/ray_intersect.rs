use nalgebra_glm::Vec3;

use crate::color::Color;
use crate::texture::Texture;

/// Como una superficie responde a la luz.
///
/// Los cuatro canales de luz se suman: color = difuso * albedo[0] + especular * albedo[1]
/// + reflejo * reflectivity + refraccion * transparency. Para que no se sobre-ilumine, la
/// suma de albedo[0], reflectivity y transparency deberia quedar cerca de 1 o menos.
#[derive(Debug, Clone, Copy)]
pub struct Material {
    /// De donde sale el color base (color solido, ajedrez o imagen)
    pub texture: Texture,
    /// Cuanta luz difusa [0] y especular [1] refleja la superficie
    pub albedo: [f32; 2],
    /// Que tan concentrado es el brillo especular: bajo = brillo grande y suave (goma),
    /// alto = punto chiquito y fuerte (espejo)
    pub specular: f32,
    /// Cuanta luz atraviesa la superficie (0 = opaco, 1 = totalmente transparente)
    pub transparency: f32,
    /// Cuanta luz rebota como en un espejo (0 = nada, 1 = espejo perfecto)
    pub reflectivity: f32,
    /// Cuanto se dobla la luz al atravesar el material (aire 1.0, agua 1.33, vidrio 1.5).
    /// Solo importa si `transparency` es mayor que 0.
    pub refractive_index: f32,
}

impl Material {
    /// Material mate y opaco con la textura dada; los demas parametros se cambian despues
    /// con `Material { specular: 50.0, ..Material::new(textura) }`
    pub fn new(texture: Texture) -> Self {
        Material {
            texture,
            albedo: [1.0, 0.0],
            specular: 32.0,
            transparency: 0.0,
            reflectivity: 0.0,
            refractive_index: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub is_intersecting: bool,
    pub material: Material,
    /// Coordenadas de textura del punto de impacto, de 0 a 1
    pub u: f32,
    pub v: f32,
}

impl Intersect {
    pub fn new(point: Vec3, normal: Vec3, distance: f32, material: Material, u: f32, v: f32) -> Self {
        Intersect {
            point,
            normal,
            distance,
            is_intersecting: true,
            material,
            u,
            v,
        }
    }

    pub fn empty() -> Self {
        Intersect {
            point: Vec3::zeros(),
            normal: Vec3::zeros(),
            distance: 0.0,
            is_intersecting: false,
            material: Material::new(Texture::Solid(Color::new(0, 0, 0))),
            u: 0.0,
            v: 0.0,
        }
    }
}

pub trait RayIntersect {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect;
}
