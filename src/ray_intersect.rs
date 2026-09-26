use nalgebra_glm::Vec3;

pub trait RayIntersect {
    // Returns the distance along the ray to the closest hit, or None if the ray misses
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<f32>;
}
