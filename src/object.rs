use crate::framebuffer::Color;
use crate::vec3::Vec3;

pub struct Intersect {
    pub distance: f32,
}

pub trait Object {
    fn intersect(&self, origin: &Vec3, direction: &Vec3) -> Option<Intersect>;
    fn color(&self) -> Color;
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub color: Color,
}

impl Object for Sphere {
    fn intersect(&self, origin: &Vec3, direction: &Vec3) -> Option<Intersect> {
        let oc = *origin - self.center;
        let a = direction.dot(direction);
        let b = 2.0 * oc.dot(direction);
        let c = oc.dot(&oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrt_d = discriminant.sqrt();
        let t1 = (-b - sqrt_d) / (2.0 * a);
        let t2 = (-b + sqrt_d) / (2.0 * a);

        let distance = if t1 > 0.001 {
            t1
        } else if t2 > 0.001 {
            t2
        } else {
            return None;
        };

        Some(Intersect { distance })
    }

    fn color(&self) -> Color {
        self.color
    }
}
