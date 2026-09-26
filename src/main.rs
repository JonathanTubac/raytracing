mod framebuffer;
mod ray_intersect;
mod render;
mod scene;
mod sphere;

use framebuffer::Framebuffer;
use minifb::{Key, Window, WindowOptions};
use render::render;
use scene::{IMG, gopher};

fn main() {
    let mut framebuffer = Framebuffer::new(IMG as usize, IMG as usize);

    let objects = gopher();

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
