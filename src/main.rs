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
mod normal_map;
mod ray_intersect;
mod render;
mod scene;
mod skybox;
// Las esferas quedaron de las primeras versiones; el diorama es solo de cubos, pero las
// pruebas del render las siguen usando
#[cfg(test)]
mod sphere;
mod texture;
mod voxel;
mod window;

use controls::Controls;
use framebuffer::Framebuffer;
use render::{render, Scene};
use scene::{camara, cielo, luces, objetos};
use std::time::{Duration, Instant};
use window::{Key, Window};

// Tiempo minimo entre cuadros (60 por segundo), para no ocupar un nucleo entero sin hacer nada
const CUADRO: Duration = Duration::from_micros(16_667);

// Mientras la camara se mueve se dibuja con pixeles de 2x2 (4 veces menos rayos) para que
// responda fluido; al soltar se dibuja el cuadro a resolucion completa
const PIXEL_EN_MOVIMIENTO: usize = 2;

fn main() {
    let mut framebuffer = Framebuffer::new(800, 600);

    // Todo el diorama es un solo objeto: la grilla de voxeles
    let world = objetos();
    let lights = luces(&world);
    let skybox = cielo();
    let mut scene = Scene {
        objects: std::slice::from_ref(&world),
        lights: &lights,
        skybox: &skybox,
        normal_maps: true,
    };
    let mut camera = camara();
    let mut controls = Controls::new();

    // `cargo run -- --captura` renderiza un solo cuadro a output.png sin abrir la ventana
    if std::env::args().any(|arg| arg == "--captura") {
        render(&mut framebuffer, &scene, &camera, 1);
        framebuffer
            .save_to_file("output.png")
            .expect("Error al guardar output.png");
        return;
    }

    // `cargo run -- --bench` mide cuanto tarda un cuadro mientras la camara da una vuelta
    if std::env::args().any(|arg| arg == "--bench") {
        const CUADROS: u32 = 30;
        let inicio = Instant::now();
        for _ in 0..CUADROS {
            camera.orbit(std::f32::consts::TAU / CUADROS as f32, 0.0);
            render(&mut framebuffer, &scene, &camera, 1);
        }
        let por_cuadro = inicio.elapsed() / CUADROS;
        println!(
            "{CUADROS} cuadros de {}x{} con {} bloques: {:.1} ms por cuadro ({:.1} FPS)",
            framebuffer.width,
            framebuffer.height,
            world.block_count(),
            por_cuadro.as_secs_f64() * 1000.0,
            1.0 / por_cuadro.as_secs_f64()
        );
        return;
    }

    let mut window = Window::new("Raytracer", framebuffer.width, framebuffer.height)
        .expect("Error al crear la ventana");
    window.set_title(&titulo(&scene));
    // Para cambiar los mapas normales una sola vez por cada vez que se aprieta la N
    let mut n_apretada = false;

    render(&mut framebuffer, &scene, &camera, 1);
    // Si el ultimo cuadro se dibujo a baja resolucion y falta el de resolucion completa
    let mut falta_detalle = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let inicio = Instant::now();

        // Solo se vuelve a renderizar cuando la camara se mueve, o cuando se deja de mover
        // para dibujar el cuadro con todo el detalle
        // N prende y apaga los mapas normales, para comparar el relieve con y sin ellos
        let n = window.is_key_down(Key::N);
        if n && !n_apretada {
            scene.normal_maps = !scene.normal_maps;
            window.set_title(&titulo(&scene));
            falta_detalle = true;
        }
        n_apretada = n;

        if controls.update(&window, &mut camera) {
            render(&mut framebuffer, &scene, &camera, PIXEL_EN_MOVIMIENTO);
            falta_detalle = true;
        } else if falta_detalle {
            render(&mut framebuffer, &scene, &camera, 1);
            falta_detalle = false;
        }

        window.update_with_buffer(framebuffer.buffer(), framebuffer.width, framebuffer.height);

        if let Some(sobra) = CUADRO.checked_sub(inicio.elapsed()) {
            std::thread::sleep(sobra);
        }
    }
}

/// Titulo de la ventana: el nombre, los controles y si los mapas normales estan prendidos
fn titulo<O>(scene: &Scene<O>) -> String {
    let mapas = if scene.normal_maps { "si" } else { "no" };
    format!("Raytracer | flechas/mouse: rotar, W/S/rueda: zoom | N: mapas normales ({mapas})")
}
