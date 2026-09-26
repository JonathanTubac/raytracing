use nalgebra_glm::{Vec3, normalize};

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::ray_intersect::RayIntersect;
use crate::sphere::Sphere;

// Color de los pixeles donde el rayo no golpea ninguna esfera
const FONDO: Color = Color::new(255, 255, 255);

// Distancia de la camara al plano de imagen. Un valor alto = FOV angosto, asi las esferas
// grandes que estan lejos del centro casi no se deforman.
pub const FOCAL: f32 = 6.0;

pub fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3, objects: &[Sphere]) -> Color {
    // Nos quedamos con el impacto mas cercano a la camara
    let mut closest = f32::INFINITY;
    let mut color = FONDO;

    for object in objects {
        let intersect = object.ray_intersect(ray_origin, ray_direction);
        if intersect.is_intersecting && intersect.distance < closest {
            closest = intersect.distance;
            color = intersect.material.diffuse;
        }
    }

    color
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Sphere]) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            // Mapea el pixel a espacio de pantalla [-1, 1]
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            // Ajuste por aspect ratio
            let screen_x = screen_x * aspect_ratio;

            // Direccion del rayo para este pixel
            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -FOCAL));

            // Se lanza el rayo y se obtiene el color
            let pixel_color = cast_ray(&Vec3::new(0.0, 0.0, 0.0), &ray_direction, objects);

            framebuffer.set_current_color(pixel_color.to_hex());
            framebuffer.point(x, y);
        }
    }
}
