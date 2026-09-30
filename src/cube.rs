use crate::math::Vec3;

#[cfg(test)]
use crate::ray_intersect::RayIntersect;
use crate::ray_intersect::{Face, Intersect, Material};

/// Axis-aligned cube (or box), defined by its minimum and maximum corners
#[cfg(test)]
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

#[cfg(test)]
impl Cube {
    /// Cube of side `size` centered at `center`
    pub fn new(center: Vec3, size: f32, material: Material) -> Self {
        let half = Vec3::new(size, size, size) * 0.5;
        Cube {
            min: center - half,
            max: center + half,
            material,
        }
    }
}

/// Hit on the face perpendicular to `axis`, on the maximum side if `positive`
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

/// (u, v) coordinates of point `p` on the face, and where u and v grow
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
    /// Slab method: the cube is the intersection of three slabs, one per axis
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        let mut t_near = f32::NEG_INFINITY;
        let mut t_far = f32::INFINITY;
        // Axis and side of the faces where the ray enters and exits
        let mut near_face = (0, false);
        let mut far_face = (0, false);

        for axis in 0..3 {
            let origin = ray_origin[axis];
            let direction = ray_direction[axis];

            if direction.abs() < 1e-8 {
                // The ray is parallel to this slab: it is either always inside it or never
                if origin < self.min[axis] || origin > self.max[axis] {
                    return Intersect::empty();
                }
                continue;
            }

            let inv = 1.0 / direction;
            let t_min = (self.min[axis] - origin) * inv;
            let t_max = (self.max[axis] - origin) * inv;

            // A ray going in the negative direction enters on the max side and exits on the min
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

        // Use the nearest hit in front of the ray
        let (distance, (axis, positive)) = if t_near > 0.0 {
            (t_near, near_face)
        } else if t_far > 0.0 {
            (t_far, far_face)
        } else {
            return Intersect::empty();
        };

        let point = ray_origin + ray_direction * distance;
        // Point position inside the cube, from 0 to 1 on each axis
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
        // The face center is the texture center
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

        // Shoot from outside each face toward the cube center
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
        // The normal still points out of the cube
        assert!(cerca(&hit.normal, &Vec3::new(0.0, 0.0, -1.0)));
    }

    #[test]
    fn la_textura_de_la_cara_no_sale_de_lado_ni_al_reves() {
        let cubo = cubo();
        // Top left corner of the front face, seen from +Z
        let hit = cubo.ray_intersect(&Vec3::new(-0.9, 0.9, 0.0), &Vec3::new(0.0, 0.0, -1.0));
        assert!(hit.u < 0.1 && hit.v < 0.1, "u = {}, v = {}", hit.u, hit.v);

        // Same on the right face (+X), seen from outside: its left is +Z
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
            // u (or v) must grow when the ray moves along the tangent (or bitangent)
            let origen = centro + eje * 5.0;
            let hit = cubo.ray_intersect(&origen, &-eje);
            let corrido_t = cubo.ray_intersect(&(origen + hit.tangent * 0.2), &-eje);
            let corrido_b = cubo.ray_intersect(&(origen + hit.bitangent * 0.2), &-eje);

            assert!(corrido_t.u > hit.u + 0.05 && (corrido_t.v - hit.v).abs() < 1e-5, "{eje:?}");
            assert!(corrido_b.v > hit.v + 0.05 && (corrido_b.u - hit.u).abs() < 1e-5, "{eje:?}");
            // Both lie on the face, perpendicular to the normal
            assert!(dot(&hit.tangent, &hit.normal).abs() < 1e-6);
            assert!(dot(&hit.bitangent, &hit.normal).abs() < 1e-6);
        }
    }

    #[test]
    fn un_rayo_diagonal_entra_por_la_cara_correcta() {
        // Steep ray toward the top edge: it must enter through the top face
        let origen = Vec3::new(0.0, 5.0, -3.5);
        let direccion = crate::math::normalize(&Vec3::new(0.0, -1.0, -0.2));
        let hit = cubo().ray_intersect(&origen, &direccion);

        assert!(hit.is_intersecting);
        assert!(cerca(&hit.normal, &Vec3::new(0.0, 1.0, 0.0)));
    }
}
