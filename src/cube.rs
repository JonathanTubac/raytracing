use crate::math::Vec3;

#[cfg(test)]
use crate::ray_intersect::RayIntersect;
use crate::ray_intersect::{Face, Intersect, Material};

/// Cubo (o caja) alineado a los ejes, definido por su esquina minima y maxima.
///
/// El diorama no usa cubos sueltos sino la grilla de voxeles (voxel.rs), que es mucho mas
/// rapida. El cubo queda como referencia: las pruebas comparan la grilla contra probar
/// cubo por cubo, y las del render lo usan para armar escenas chicas.
#[cfg(test)]
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

#[cfg(test)]
impl Cube {
    /// Cubo de lado `size` centrado en `center`
    pub fn new(center: Vec3, size: f32, material: Material) -> Self {
        let half = Vec3::new(size, size, size) * 0.5;
        Cube {
            min: center - half,
            max: center + half,
            material,
        }
    }
}

/// Impacto sobre una cara de un bloque: la perpendicular al eje `axis` (0 = x, 1 = y,
/// 2 = z), del lado maximo de ese eje si `positive`. `local` es el punto de impacto dentro
/// del bloque, de 0 a 1 en cada eje. Lo usan los cubos sueltos y la grilla de voxeles.
pub fn face_hit(
    point: Vec3,
    distance: f32,
    axis: usize,
    positive: bool,
    local: &Vec3,
    material: Material,
) -> Intersect {
    let mut normal = Vec3::zeros();
    normal[axis] = if positive { 1.0 } else { -1.0 };

    let (u, v, tangent, bitangent) = face_uv(local, &normal);
    let face = match (axis, positive) {
        (1, true) => Face::Top,
        (1, false) => Face::Bottom,
        _ => Face::Side,
    };

    Intersect::new(point, normal, distance, material, u, v).on_face(face, tangent, bitangent)
}

/// Coordenadas de textura del punto `p` (de 0 a 1 dentro del bloque) sobre la cara con esa
/// normal, y las direcciones en el mundo hacia donde crecen u y v. Cada cara recibe la
/// textura completa, vista desde afuera y derecha: v = 0 arriba y u = 0 a la izquierda.
fn face_uv(p: &Vec3, normal: &Vec3) -> (f32, f32, Vec3, Vec3) {
    let x = Vec3::new(1.0, 0.0, 0.0);
    let y = Vec3::new(0.0, 1.0, 0.0);
    let z = Vec3::new(0.0, 0.0, 1.0);

    if normal.x > 0.5 {
        (1.0 - p.z, 1.0 - p.y, -z, -y)
    } else if normal.x < -0.5 {
        (p.z, 1.0 - p.y, z, -y)
    } else if normal.z > 0.5 {
        (p.x, 1.0 - p.y, x, -y)
    } else if normal.z < -0.5 {
        (1.0 - p.x, 1.0 - p.y, -x, -y)
    } else if normal.y > 0.5 {
        (p.x, p.z, x, z)
    } else {
        (p.x, 1.0 - p.z, x, -z)
    }
}

#[cfg(test)]
impl RayIntersect for Cube {
    /// Metodo de los "slabs": el cubo es la interseccion de tres franjas, una por eje.
    /// El rayo esta dentro del cubo entre el ultimo momento en que entra a una franja
    /// (`t_near`) y el primero en que sale de alguna (`t_far`).
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let mut t_near = f32::NEG_INFINITY;
        let mut t_far = f32::INFINITY;
        // Eje y lado de la cara por donde entra y por donde sale el rayo
        let mut near_face = (0, false);
        let mut far_face = (0, false);

        for axis in 0..3 {
            let origin = ray_origin[axis];
            let direction = ray_direction[axis];

            if direction.abs() < 1e-8 {
                // El rayo va paralelo a esta franja: o siempre esta dentro de ella o nunca
                if origin < self.min[axis] || origin > self.max[axis] {
                    return Intersect::empty();
                }
                continue;
            }

            let inv = 1.0 / direction;
            let t_min = (self.min[axis] - origin) * inv;
            let t_max = (self.max[axis] - origin) * inv;

            // Si el rayo va en sentido negativo entra por el lado max y sale por el min
            let (t_enter, t_exit, enters_max) = if inv >= 0.0 {
                (t_min, t_max, false)
            } else {
                (t_max, t_min, true)
            };

            if t_enter > t_near {
                t_near = t_enter;
                near_face = (axis, enters_max);
            }
            if t_exit < t_far {
                t_far = t_exit;
                far_face = (axis, !enters_max);
            }

            if t_near > t_far {
                return Intersect::empty();
            }
        }

        // Se usa el impacto mas cercano delante del rayo. Si el origen esta dentro del
        // cubo (un rayo refractado, por ejemplo), el impacto es la cara por donde sale.
        let (distance, (axis, positive)) = if t_near > 0.0 {
            (t_near, near_face)
        } else if t_far > 0.0 {
            (t_far, far_face)
        } else {
            return Intersect::empty();
        };

        let point = ray_origin + ray_direction * distance;
        // Posicion del punto dentro del cubo, de 0 a 1 en cada eje
        let local = (point - self.min).component_div(&(self.max - self.min));

        face_hit(point, distance, axis, positive, &local, self.material)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::math::dot;
    use crate::texture::Texture;

    fn cubo() -> Cube {
        Cube::new(
            Vec3::new(0.0, 0.0, -5.0),
            2.0,
            Material::new(Texture::Solid(Color::new(1, 2, 3))),
        )
    }

    fn cerca(a: &Vec3, b: &Vec3) -> bool {
        (a - b).norm() < 1e-5
    }

    #[test]
    fn el_impacto_de_frente_da_distancia_punto_normal_y_uv() {
        let hit = cubo().ray_intersect(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0));

        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.0).abs() < 1e-5);
        assert!(cerca(&hit.point, &Vec3::new(0.0, 0.0, -4.0)));
        assert!(cerca(&hit.normal, &Vec3::new(0.0, 0.0, 1.0)));
        // El centro de la cara es el centro de la textura
        assert!((hit.u - 0.5).abs() < 1e-5);
        assert!((hit.v - 0.5).abs() < 1e-5);
    }

    #[test]
    fn cada_cara_devuelve_su_normal_hacia_afuera() {
        let cubo = cubo();
        let centro = Vec3::new(0.0, 0.0, -5.0);
        let ejes = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ];

        // Se dispara desde afuera de cada cara hacia el centro del cubo
        for eje in ejes {
            let hit = cubo.ray_intersect(&(centro + eje * 5.0), &-eje);
            assert!(hit.is_intersecting);
            assert!(cerca(&hit.normal, &eje), "cara {eje:?} dio normal {:?}", hit.normal);
            assert!((hit.distance - 4.0).abs() < 1e-5);
        }
    }

    #[test]
    fn un_rayo_que_pasa_al_lado_no_choca() {
        let hit = cubo().ray_intersect(&Vec3::new(1.5, 0.0, 0.0), &Vec3::new(0.0, 0.0, -1.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn un_cubo_detras_del_rayo_no_choca() {
        let hit = cubo().ray_intersect(&Vec3::zeros(), &Vec3::new(0.0, 0.0, 1.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn desde_adentro_choca_con_la_cara_de_salida() {
        let hit = cubo().ray_intersect(&Vec3::new(0.0, 0.0, -5.0), &Vec3::new(0.0, 0.0, -1.0));

        assert!(hit.is_intersecting);
        assert!((hit.distance - 1.0).abs() < 1e-5);
        // La normal sigue apuntando hacia afuera del cubo
        assert!(cerca(&hit.normal, &Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn la_textura_de_la_cara_no_sale_de_lado_ni_al_reves() {
        let cubo = cubo();
        // Esquina de arriba a la izquierda de la cara del frente, vista desde +Z
        let hit = cubo.ray_intersect(&Vec3::new(-0.9, 0.9, 0.0), &Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.u < 0.1 && hit.v < 0.1, "u = {}, v = {}", hit.u, hit.v);

        // Lo mismo en la cara derecha (+X), vista desde afuera: su izquierda es +Z
        let hit = cubo.ray_intersect(&Vec3::new(5.0, 0.9, -4.1), &Vec3::new(-1.0, 0.0, 0.0));
        assert!(hit.u < 0.1 && hit.v < 0.1, "u = {}, v = {}", hit.u, hit.v);
    }

    #[test]
    fn distingue_la_cara_de_arriba_la_de_abajo_y_los_lados() {
        let cubo = cubo();
        let centro = Vec3::new(0.0, 0.0, -5.0);
        let cara = |eje: Vec3| cubo.ray_intersect(&(centro + eje * 5.0), &-eje).face;

        assert_eq!(cara(Vec3::new(0.0, 1.0, 0.0)), Face::Top);
        assert_eq!(cara(Vec3::new(0.0, -1.0, 0.0)), Face::Bottom);
        assert_eq!(cara(Vec3::new(1.0, 0.0, 0.0)), Face::Side);
        assert_eq!(cara(Vec3::new(0.0, 0.0, -1.0)), Face::Side);
    }

    #[test]
    fn la_tangente_y_la_bitangente_apuntan_hacia_donde_crecen_u_y_v() {
        let cubo = cubo();
        let centro = Vec3::new(0.0, 0.0, -5.0);
        let ejes = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ];

        for eje in ejes {
            // Rayo hacia el centro de la cara, y otro corrido un poco sobre la tangente
            // (o la bitangente): u (o v) tiene que crecer y lo otro quedar igual
            let origen = centro + eje * 5.0;
            let hit = cubo.ray_intersect(&origen, &-eje);
            let corrido_t = cubo.ray_intersect(&(origen + hit.tangent * 0.2), &-eje);
            let corrido_b = cubo.ray_intersect(&(origen + hit.bitangent * 0.2), &-eje);

            assert!(corrido_t.u > hit.u + 0.05 && (corrido_t.v - hit.v).abs() < 1e-5, "{eje:?}");
            assert!(corrido_b.v > hit.v + 0.05 && (corrido_b.u - hit.u).abs() < 1e-5, "{eje:?}");
            // Las dos quedan sobre la cara, perpendiculares a la normal
            assert!(dot(&hit.tangent, &hit.normal).abs() < 1e-6);
            assert!(dot(&hit.bitangent, &hit.normal).abs() < 1e-6);
        }
    }

    #[test]
    fn un_rayo_diagonal_entra_por_la_cara_correcta() {
        // Desde arriba y adelante, apuntando al borde superior del frente pero mas empinado:
        // tiene que entrar por la cara de arriba
        let origen = Vec3::new(0.0, 5.0, -3.5);
        let direccion = crate::math::normalize(&Vec3::new(0.0, -1.0, -0.2));
        let hit = cubo().ray_intersect(&origen, &direccion);

        assert!(hit.is_intersecting);
        assert!(cerca(&hit.normal, &Vec3::new(0.0, 1.0, 0.0)));
    }
}
