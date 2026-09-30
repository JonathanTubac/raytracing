use crate::math::Vec3;

use crate::color::Color;
use crate::normal_map::NormalMap;
use crate::texture::Texture;

/// How a surface responds to light
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material {
    /// Where the base color comes from (solid color, checkerboard or image)
    pub texture: Texture,
    /// How much diffuse [0] and specular [1] light the surface reflects
    pub albedo: [f32; 2],
    /// Specular exponent: low = large and soft highlight, high = small and sharp
    pub specular: f32,
    /// How much light goes through the surface (0 = opaque, 1 = fully transparent)
    pub transparency: f32,
    /// How much light bounces like a mirror (0 = none, 1 = perfect mirror)
    pub reflectivity: f32,
    /// How much light bends going through the material (air 1.0, water 1.33, glass 1.5)
    pub refractive_index: f32,
    /// Surface relief: changes the normal point by point (`None` = flat face)
    pub normal_map: Option<NormalMap>,
    /// Light emitted by the surface, as a multiple of its texture color (0 = none)
    pub emission: f32,
}

impl Material {
    /// Matte, opaque material; change the rest with `..Material::new(textura)`
    pub fn new(texture: Texture) -> Self {
        Material {
            texture,
            albedo: [1.0, 0.0],
            specular: 32.0,
            transparency: 0.0,
            reflectivity: 0.0,
            refractive_index: 1.0,
            normal_map: None,
            emission: 0.0,
        }
    }

    /// Whether that texture pixel has no surface: the gap between the leaves of a tree
    pub fn is_hole(&self, alpha: Option<f32>) -> bool {
        self.transparency == 0.0 && alpha.is_some_and(|a| a < 0.5)
    }

    /// How much light the surface lets through at a texture pixel with that opacity
    pub fn transparency_at(&self, alpha: Option<f32>) -> f32 {
        match alpha {
            Some(a) if a >= 1.0 => 0.0,
            _ => self.transparency,
        }
    }
}

/// Which block face was hit, to pick its texture. Spheres are all "side".
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Face {
    Top,
    Side,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub struct Intersect {
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
    pub is_intersecting: bool,
    pub material: Material,
    /// Texture coordinates of the hit point, from 0 to 1
    pub u: f32,
    pub v: f32,
    pub face: Face,
    /// World directions in which u and v grow along the surface
    pub tangent: Vec3,
    pub bitangent: Vec3,
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
            face: Face::Side,
            tangent: Vec3::zeros(),
            bitangent: Vec3::zeros(),
        }
    }

    /// The same hit, on the given face and with its u and v directions
    pub fn on_face(self, face: Face, tangent: Vec3, bitangent: Vec3) -> Self {
        Intersect {
            face,
            tangent,
            bitangent,
            ..self
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
            face: Face::Side,
            tangent: Vec3::zeros(),
            bitangent: Vec3::zeros(),
        }
    }
}

/// Anything a ray can hit
pub trait RayIntersect: Sync {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect;
}

/// Allows mixing different object types in one scene (`Vec<Box<dyn RayIntersect>>`)
impl<T: RayIntersect + ?Sized> RayIntersect for Box<T> {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        (**self).ray_intersect(ray_origin, ray_direction)
    }
}
