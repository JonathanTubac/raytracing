mod blocks;
mod camera;
mod color;
mod controls;
mod cube;
mod framebuffer;
mod image_io;
mod inflate;
mod light;
mod math;
mod ray_intersect;
mod render;
mod scene;
mod skybox;
// Las esferas quedaron de las primeras versiones; el diorama es solo de cubos, pero las
// pruebas del render las siguen usando
#[cfg(test)]
mod sphere;
mod texture;
mod window;

use controls::Controls;
use framebuffer::Framebuffer;
use render::{render, Scene};
use scene::{camara, cielo, luces, objetos};
use std::time::{Duration, Instant};
use window::{Key, Window};

// Tiempo minimo entre cuadros (60 por segundo), para no ocupar un nucleo entero sin hacer nada
const CUADRO: Duration = Duration::from_micros(16_667);

fn main() {
    let mut framebuffer = Framebuffer::new(800, 600);

    let objects = objetos();
    let lights = luces();
    let skybox = cielo();
    let scene = Scene {
        objects: &objects,
        lights: &lights,
        skybox: &skybox,
    };
    let mut camera = camara();
    let mut controls = Controls::new();

    // `cargo run -- --captura` renderiza un solo cuadro a output.png sin abrir la ventana
    if std::env::args().any(|arg| arg == "--captura") {
        render(&mut framebuffer, &scene, &camera);
        framebuffer
            .save_to_file("output.png")
            .expect("Error al guardar output.png");
        return;
    }

    let mut window = Window::new("Raytracer", framebuffer.width, framebuffer.height)
        .expect("Error al crear la ventana");

    render(&mut framebuffer, &scene, &camera);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let inicio = Instant::now();

        // Solo se vuelve a renderizar cuando la camara se mueve
        if controls.update(&window, &mut camera) {
            render(&mut framebuffer, &scene, &camera);
        }

        window.update_with_buffer(framebuffer.buffer(), framebuffer.width, framebuffer.height);

        if let Some(sobra) = CUADRO.checked_sub(inicio.elapsed()) {
            std::thread::sleep(sobra);
        }
    }
}
