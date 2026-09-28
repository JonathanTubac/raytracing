mod camera;
mod color;
mod controls;
mod cube;
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
use scene::{camara, luz, objetos};

fn main() {
    let mut framebuffer = Framebuffer::new(800, 600);

    let objects = objetos();
    let light = luz();
    let mut camera = camara();
    let mut controls = Controls::new();

    // `cargo run -- --captura` renderiza un solo cuadro a output.png sin abrir la ventana
    if std::env::args().any(|arg| arg == "--captura") {
        render(&mut framebuffer, &objects, &camera, &light);
        framebuffer
            .save_to_file("output.png")
            .expect("Error al guardar output.png");
        return;
    }

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
