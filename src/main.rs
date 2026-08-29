mod framebuffer;
mod object;
mod vec3;

use framebuffer::{Color, Framebuffer};
use object::{Object, Sphere};
use vec3::{normalize, Vec3};

const BACKGROUND_COLOR: Color = Color { r: 20, g: 20, b: 30 };

fn cast_ray(origin: &Vec3, direction: &Vec3, objects: &[Box<dyn Object>]) -> Color {
    let mut closest_distance = f32::INFINITY;
    let mut pixel_color = BACKGROUND_COLOR;

    for object in objects {
        if let Some(intersect) = object.intersect(origin, direction) {
            if intersect.distance < closest_distance {
                closest_distance = intersect.distance;
                pixel_color = object.color();
            }
        }
    }

    pixel_color
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Box<dyn Object>]) {
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

    let objects: Vec<Box<dyn Object>> = vec![
        Box::new(Sphere {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.0,
            color: Color::new(200, 60, 60),
        }),
        Box::new(Sphere {
            center: Vec3::new(-2.0, 0.5, -6.0),
            radius: 1.3,
            color: Color::new(60, 120, 200),
        }),
        Box::new(Sphere {
            center: Vec3::new(1.5, -0.5, -4.0),
            radius: 0.6,
            color: Color::new(80, 200, 100),
        }),
    ];

    render(&mut framebuffer, &objects);

    framebuffer
        .save_to_file("output.png")
        .expect("no se pudo guardar la imagen");

    println!(
        "Listo: output.png generado ({}x{})",
        framebuffer.width, framebuffer.height
    );
}
