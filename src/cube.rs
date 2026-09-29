use crate::math::Vec3;

use crate::ray_intersect::{Face, Intersect, Material, RayIntersect};

/// Cubo (o caja) alineado a los ejes, definido por su esquina minima y maxima
pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

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

    /// Normal hacia afuera de la cara perpendicular al eje `axis` (0 = x, 1 = y, 2 = z).
    /// `positive` indica si es la cara del lado `max` de ese eje.
    fn face_normal(axis: usize, positive: bool) -> Vec3 {
        let mut normal = Vec3::zeros();
        normal[axis] = if positive { 1.0 } else { -1.0 };
        normal
    }

    /// Coordenadas de textura del punto sobre la cara con esa normal. Cada cara recibe la
    /// textura completa, vista desde afuera y derecha: v = 0 arriba y u = 0 a la izquierda.
    fn face_uv(&self, point: &Vec3, normal: &Vec3) -> (f32, f32) {
        // Posicion del punto dentro del cubo, de 0 a 1 en cada eje
        let size = self.max - self.min;
        let p = (point - self.min).component_div(&size);

        if normal.x > 0.5 {
            (1.0 - p.z, 1.0 - p.y)
        } else if normal.x < -0.5 {
            (p.z, 1.0 - p.y)
        } else if normal.z > 0.5 {
            (p.x, 1.0 - p.y)
        } else if normal.z < -0.5 {
            (1.0 - p.x, 1.0 - p.y)
        } else if normal.y > 0.5 {
            (p.x, p.z)
        } else {
            (p.x, 1.0 - p.z)
        }
    }
}

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
        let normal = Cube::face_normal(axis, positive);
        let (u, v) = self.face_uv(&point, &normal);
        let face = match (axis, positive) {
            (1, true) => Face::Top,
            (1, false) => Face::Bottom,
            _ => Face::Side,
        };

        Intersect::new(point, normal, distance, self.material, u, v).on_face(face)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
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
