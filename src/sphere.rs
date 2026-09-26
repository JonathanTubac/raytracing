use std::f32::consts::PI;

use nalgebra_glm::{dot, normalize, Vec3};

use crate::ray_intersect::{Intersect, Material, RayIntersect};

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        // Vector from the ray origin to the center of the sphere
        let oc = ray_origin - self.center;

        // Coefficients for the quadratic equation
        // a = dot(ray_direction, ray_direction)
        // This is the dot product of the ray direction with itself, representing the squared length of the direction vector.
        let a = dot(ray_direction, ray_direction);

        // b = 2.0 * dot(oc, ray_direction)
        // This is twice the dot product of the vector oc and the ray direction.
        // It represents the projection of oc onto the ray direction, scaled by 2.
        let b = 2.0 * dot(&oc, ray_direction);

        // c = dot(oc, oc) - radius^2
        // This is the dot product of oc with itself minus the squared radius of the sphere.
        // It represents the squared distance from the ray origin to the sphere center minus the squared radius.
        let c = dot(&oc, &oc) - self.radius * self.radius;

        // Discriminant of the quadratic equation
        // discriminant = b^2 - 4ac
        // The discriminant determines the number of solutions to the quadratic equation.
        // If the discriminant is greater than zero, the ray intersects the sphere at two points.
        // If the discriminant is zero, the ray is tangent to the sphere and intersects at one point.
        // If the discriminant is less than zero, the ray does not intersect the sphere.
        let discriminant = b * b - 4.0 * a * c;

        // The ray intersects the sphere if the discriminant is greater than zero
        if discriminant <= 0.0 {
            return Intersect::empty();
        }

        // The two solutions are the distances to the near and far points of the sphere.
        // We keep the nearest one that is in front of the ray origin.
        let sqrt_d = discriminant.sqrt();
        let t_near = (-b - sqrt_d) / (2.0 * a);
        let t_far = (-b + sqrt_d) / (2.0 * a);

        let distance = if t_near > 0.0 {
            t_near
        } else if t_far > 0.0 {
            t_far
        } else {
            return Intersect::empty();
        };

        // Point where the ray hits the sphere and the surface normal at that point
        let point = ray_origin + ray_direction * distance;
        let normal = normalize(&(point - self.center));

        // Coordenadas de textura: u es el angulo alrededor del eje vertical y v va de polo a polo
        let u = 0.5 + normal.z.atan2(normal.x) / (2.0 * PI);
        let v = 0.5 - normal.y.clamp(-1.0, 1.0).asin() / PI;

        Intersect::new(point, normal, distance, self.material, u, v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::texture::Texture;

    fn esfera() -> Sphere {
        Sphere {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.0,
            material: Material::new(Texture::Solid(Color::new(1, 2, 3))),
        }
    }

    #[test]
    fn el_impacto_de_frente_da_distancia_punto_normal_y_uv() {
        let hit = esfera().ray_intersect(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0));

        assert!(hit.is_intersecting);
        assert!((hit.distance - 4.0).abs() < 1e-5);
        assert!((hit.point - Vec3::new(0.0, 0.0, -4.0)).norm() < 1e-5);
        assert!((hit.normal - Vec3::new(0.0, 0.0, 1.0)).norm() < 1e-5);
        // De frente cae en el "ecuador" (v = 0.5), a un cuarto de vuelta (u = 0.75)
        assert!((hit.u - 0.75).abs() < 1e-5);
        assert!((hit.v - 0.5).abs() < 1e-5);
    }

    #[test]
    fn un_rayo_que_falla_devuelve_un_intersect_vacio() {
        let hit = esfera().ray_intersect(&Vec3::zeros(), &Vec3::new(0.0, 1.0, 0.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn los_polos_tienen_v_0_arriba_y_1_abajo() {
        let arriba = esfera().ray_intersect(&Vec3::new(0.0, 5.0, -5.0), &Vec3::new(0.0, -1.0, 0.0));
        let abajo = esfera().ray_intersect(&Vec3::new(0.0, -5.0, -5.0), &Vec3::new(0.0, 1.0, 0.0));

        assert!(arriba.v.abs() < 1e-5);
        assert!((abajo.v - 1.0).abs() < 1e-5);
    }
}
