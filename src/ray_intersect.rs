use crate::math::Vec3;

use crate::color::Color;
use crate::normal_map::NormalMap;
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
    /// Relieve de la superficie: cambia la normal punto por punto (`None` = cara lisa)
    pub normal_map: Option<NormalMap>,
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
            normal_map: None,
        }
    }

    /// Si en ese pixel de la textura no hay superficie: el hueco entre las hojas de un
    /// arbol. El rayo sigue de largo como si no hubiera chocado. Solo aplica a materiales
    /// opacos; en los transparentes (vidrio, agua) la opacidad cambia cuanto se ve a traves.
    pub fn is_hole(&self, alpha: Option<f32>) -> bool {
        self.transparency == 0.0 && alpha.is_some_and(|a| a < 0.5)
    }

    /// Cuanta luz deja pasar la superficie en un pixel de la textura con esa opacidad.
    /// Si la textura no trae opacidad, se usa `transparency` en toda la superficie. Si la
    /// trae, los pixeles totalmente opacos (el marco del vidrio) no dejan pasar nada y el
    /// resto usa `transparency`.
    pub fn transparency_at(&self, alpha: Option<f32>) -> f32 {
        match alpha {
            Some(a) if a >= 1.0 => 0.0,
            _ => self.transparency,
        }
    }
}

/// Que cara de un bloque se golpeo, para elegir su textura. Las esferas son todo "lado".
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
    /// Coordenadas de textura del punto de impacto, de 0 a 1
    pub u: f32,
    pub v: f32,
    pub face: Face,
    /// Direcciones en el mundo hacia donde crecen u y v sobre la superficie. Con ellas y
    /// la normal se orienta el mapa normal de la cara.
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

    /// El mismo impacto, sobre la cara indicada y con sus direcciones de u y v
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

/// Todo lo que un rayo puede golpear. Es `Sync` porque los pixeles se calculan en paralelo
/// y todos los hilos leen la misma escena.
pub trait RayIntersect: Sync {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect;
}

/// Permite mezclar objetos de distinto tipo en una misma escena (`Vec<Box<dyn RayIntersect>>`)
impl<T: RayIntersect + ?Sized> RayIntersect for Box<T> {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        (**self).ray_intersect(ray_origin, ray_direction)
    }
}
