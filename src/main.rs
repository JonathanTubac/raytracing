mod framebuffer;
mod ray_intersect;

use framebuffer::Framebuffer;
use nalgebra_glm::{Vec3, normalize};
use ray_intersect::{RayIntersect, Sphere};
use minifb::{Key, Window, WindowOptions};

pub fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3, objects: &[Sphere]) -> u32 {
    for object in objects {
        if object.ray_intersect(ray_origin, ray_direction) {
            return 0xFFFFFF;
        }
    }

    0x000000
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
            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));

            // Se lanza el rayo y se obtiene el color
            let pixel_color = cast_ray(&Vec3::new(0.0, 0.0, 0.0), &ray_direction, objects);

            framebuffer.set_current_color(pixel_color);
            framebuffer.point(x, y);
        }
    }
}

fn main() {
    let mut framebuffer = Framebuffer::new(800, 600);

    let objects = vec![
        Sphere {center: Vec3::new(0.0, 0.0, -5.0), radius: 1.0,},

        
    ];

    render(&mut framebuffer, &objects);

    let mut window = Window::new(
        "Raytracer",
        framebuffer.width,
        framebuffer.height,
        WindowOptions::default(),
    )
    .expect("Error al crear la ventana");

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        window.update_with_buffer(framebuffer.buffer(), framebuffer.width, framebuffer.height).expect("Error al crear la ventana");
    }
}
