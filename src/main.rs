mod camera;
mod color;
mod controls;
mod cube;
mod framebuffer;
mod image_io;
mod light;
mod math;
mod ray_intersect;
mod render;
mod scene;
mod sphere;
mod texture;
mod window;

use controls::Controls;
use framebuffer::Framebuffer;
use render::render;
use scene::{camara, luces, objetos};
use std::time::{Duration, Instant};
use window::{Key, Window};

// Tiempo minimo entre cuadros (60 por segundo), para no ocupar un nucleo entero sin hacer nada
const CUADRO: Duration = Duration::from_micros(16_667);

fn main() {
    let mut framebuffer = Framebuffer::new(800, 600);

    let objects = objetos();
    let lights = luces();
    let mut camera = camara();
    let mut controls = Controls::new();

    // `cargo run -- --captura` renderiza un solo cuadro a output.png sin abrir la ventana
    if std::env::args().any(|arg| arg == "--captura") {
        render(&mut framebuffer, &objects, &camera, &lights);
        framebuffer
            .save_to_file("output.png")
            .expect("Error al guardar output.png");
        return;
    }

    let mut window = Window::new("Raytracer", framebuffer.width, framebuffer.height)
        .expect("Error al crear la ventana");

    render(&mut framebuffer, &objects, &camera, &lights);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let inicio = Instant::now();

        // Solo se vuelve a renderizar cuando la camara se mueve
        if controls.update(&window, &mut camera) {
            render(&mut framebuffer, &objects, &camera, &lights);
        }

        window.update_with_buffer(framebuffer.buffer(), framebuffer.width, framebuffer.height);

        if let Some(sobra) = CUADRO.checked_sub(inicio.elapsed()) {
            std::thread::sleep(sobra);
        }
    }
}
