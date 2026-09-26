use nalgebra_glm::{dot, Vec3};

pub trait RayIntersect {
    // Returns the distance along the ray to the closest hit, or None if the ray misses
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<f32>;
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: u32,
}

impl RayIntersect for Sphere {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Option<f32> {
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
            return None;
        }

        // The two solutions are the distances to the near and far points of the sphere.
        // We keep the nearest one that is in front of the ray origin.
        let sqrt_d = discriminant.sqrt();
        let t_near = (-b - sqrt_d) / (2.0 * a);
        let t_far = (-b + sqrt_d) / (2.0 * a);

        if t_near > 0.0 {
            Some(t_near)
        } else if t_far > 0.0 {
            Some(t_far)
        } else {
            None
        }
    }
}
