mod camera;
mod color;
mod controls;
mod framebuffer;
mod light;
mod ray_intersect;
mod render;
mod scene;
mod sphere;
mod texture;

use controls::Controls;
use framebuffer::Framebuffer;
use minifb::{Key, Window, WindowOptions};
use render::render;
use scene::{camara, esferas, luz};

fn main() {
    let mut framebuffer = Framebuffer::new(800, 600);

    let objects = esferas();
    let light = luz();
    let mut camera = camara();
    let mut controls = Controls::new();

    let mut window = Window::new(
        "Raytracer",
        framebuffer.width,
        framebuffer.height,
        WindowOptions::default(),
    )
    .expect("Error al crear la ventana");

    window.set_target_fps(60);

    render(&mut framebuffer, &objects, &camera, &light);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Solo se vuelve a renderizar cuando la camara se mueve
        if controls.update(&window, &mut camera) {
            render(&mut framebuffer, &objects, &camera, &light);
        }

        window
            .update_with_buffer(framebuffer.buffer(), framebuffer.width, framebuffer.height)
            .expect("Error al actualizar la ventana");
    }
}
