mod blocks;
mod camera;
mod color;
mod controls;
mod cube;
mod diorama;
mod framebuffer;
mod grabacion;
mod image_io;
mod inflate;
mod light;
mod math;
mod nether;
mod normal_map;
mod ray_intersect;
mod render;
mod scene;
mod skybox;
// Only used by the render tests
#[cfg(test)]
mod sphere;
mod texto;
mod texture;
mod viaje;
mod voxel;
mod window;

use blocks::Blocks;
use controls::Controls;
use framebuffer::Framebuffer;
use render::render;
use scene::{inframundo, superficie, Mundo};
use std::time::{Duration, Instant};
use viaje::{Paso, Viaje};
use window::{Key, Window};

// Minimum time between frames (60 per second), so an idle loop doesn't burn a whole core
const CUADRO: Duration = Duration::from_micros(16_667);

// 2 x 2 pixels while the camera moves, so it stays responsive
const PIXEL_EN_MOVIMIENTO: usize = 2;

// Automatic rotation (R): one turn every 30 seconds
const ROTACION_AUTOMATICA: f32 = std::f32::consts::TAU / 30.0;

/// Key that acts only once per press
#[derive(Default)]
struct Tecla {
    apretada: bool,
}

impl Tecla {
    /// Whether the key was just pressed this frame
    fn pulsada(&mut self, window: &Window, key: Key) -> bool {
        let down = window.is_key_down(key);
        let pulsada = down && !self.apretada;
        self.apretada = down;
        pulsada
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |name: &str| args.iter().any(|arg| arg == name);

    let mut framebuffer = Framebuffer::new(800, 600);

    // Both worlds are built at startup, sharing textures that are loaded only once
    let blocks = Blocks::load();
    let mut mundos = [superficie(&blocks), inframundo(&blocks)];
    let mut actual = if flag("--nether") { 1 } else { 0 };
    mundos[0].horario = if flag("--noche") {
        1
    } else if flag("--dia") {
        2
    } else {
        0
    };
    // `--sin-normales` starts with normal maps off (like pressing N)
    let mut normal_maps = !flag("--sin-normales");
    let mut camera = mundos[actual].home;

    // `cargo run -- --captura` renders a single frame to output.png without opening the window
    if flag("--captura") {
        render(&mut framebuffer, &mundos[actual].scene(normal_maps), &camera, 1);
        framebuffer
            .save_to_file("output.png")
            .expect("Error al guardar output.png");
        return;
    }

    // `--grabar`: writes the tour video for ffmpeg (see grabacion.rs)
    if flag("--grabar") {
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        grabacion::grabar(&mut mundos, &mut out).expect("Error al escribir el video");
        return;
    }

    // `cargo run -- --bench` measures how long a frame takes while the camera turns around
    if flag("--bench") {
        const CUADROS: u32 = 30;
        let scene = mundos[actual].scene(normal_maps);
        let inicio = Instant::now();
        for _ in 0..CUADROS {
            camera.orbit(std::f32::consts::TAU / CUADROS as f32, 0.0);
            render(&mut framebuffer, &scene, &camera, 1);
        }
        let por_cuadro = inicio.elapsed() / CUADROS;
        println!(
            "{CUADROS} cuadros de {}x{} con {} bloques y {} luces: \
             {:.1} ms por cuadro ({:.1} FPS)",
            framebuffer.width,
            framebuffer.height,
            mundos[actual].world.block_count(),
            scene.lights.len(),
            por_cuadro.as_secs_f64() * 1000.0,
            1.0 / por_cuadro.as_secs_f64()
        );
        return;
    }

    // `--viaje`: saves frames of the portal trip
    if flag("--viaje") {
        let mut viaje = Viaje::new(&camera);
        let mut reloj = 0.0;
        for cuadro in 0.. {
            let dt = 1.0 / 30.0;
            reloj += dt;
            let mundo = &mundos[actual];
            match viaje.avanzar(dt, &mut camera, mundo.portal, &mundo.home) {
                Paso::CambiarMundo => actual = 1 - actual,
                Paso::Termino => break,
                Paso::Sigue => {}
            }
            if cuadro % 15 == 0 {
                render(&mut framebuffer, &mundos[actual].scene(normal_maps), &camera, 1);
                framebuffer.portal_swirl(viaje.overlay(), reloj);
                framebuffer
                    .save_to_file(&format!("viaje_{:02}.png", cuadro / 15))
                    .expect("Error al guardar el cuadro");
            }
        }
        return;
    }

    let mut controls = Controls::new();
    let mut window = Window::new("Raytracer", framebuffer.width, framebuffer.height)
        .expect("Error al crear la ventana");
    let mut rotando = false;
    window.set_title(&titulo(&mundos[actual], normal_maps, rotando));

    let mut tecla_n = Tecla::default();
    let mut tecla_p = Tecla::default();
    let mut tecla_r = Tecla::default();
    let mut tecla_t = Tecla::default();
    let mut viaje: Option<Viaje> = None;
    let mut reloj = 0.0;
    let mut anterior = Instant::now();

    render(&mut framebuffer, &mundos[actual].scene(normal_maps), &camera, 1);
    // Whether the last frame was drawn at low resolution and the full one is still missing
    let mut falta_detalle = false;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let inicio = Instant::now();
        let dt = inicio.duration_since(anterior).as_secs_f32();
        anterior = inicio;
        reloj += dt;

        // N toggles normal maps, to compare the relief with and without them
        if tecla_n.pulsada(&window, Key::N) {
            normal_maps = !normal_maps;
            falta_detalle = true;
        }
        // T changes the time of day (in the mine: sunset, night, day)
        if tecla_t.pulsada(&window, Key::T) && mundos[actual].siguiente_horario() {
            falta_detalle = true;
        }
        // R toggles the automatic rotation around the diorama
        if tecla_r.pulsada(&window, Key::R) {
            rotando = !rotando;
        }
        // P goes through the portal to the other world
        if tecla_p.pulsada(&window, Key::P) && viaje.is_none() {
            viaje = Some(Viaje::new(&camera));
        }
        window.set_title(&titulo(&mundos[actual], normal_maps, rotando));

        if let Some(v) = &mut viaje {
            // During the trip the animation moves the camera, not the controls
            let mundo = &mundos[actual];
            match v.avanzar(dt, &mut camera, mundo.portal, &mundo.home) {
                Paso::CambiarMundo => actual = 1 - actual,
                Paso::Termino => viaje = None,
                Paso::Sigue => {}
            }
            let overlay = viaje.as_ref().map_or(0.0, |v| v.overlay());
            let scene = mundos[actual].scene(normal_maps);
            render(&mut framebuffer, &scene, &camera, PIXEL_EN_MOVIMIENTO);
            framebuffer.portal_swirl(overlay, reloj);
            falta_detalle = true;
        } else if controls.update(&window, &mut camera) | rotando {
            // Render only when the camera moves, or for the final full detail frame
            if rotando {
                camera.orbit(ROTACION_AUTOMATICA * dt, 0.0);
            }
            let scene = mundos[actual].scene(normal_maps);
            render(&mut framebuffer, &scene, &camera, PIXEL_EN_MOVIMIENTO);
            falta_detalle = true;
        } else if falta_detalle {
            render(&mut framebuffer, &mundos[actual].scene(normal_maps), &camera, 1);
            falta_detalle = false;
        }

        window.update_with_buffer(framebuffer.buffer(), framebuffer.width, framebuffer.height);

        if let Some(sobra) = CUADRO.checked_sub(inicio.elapsed()) {
            std::thread::sleep(sobra);
        }
    }
}

/// Window title: the world and its time of day, the controls and the state of each option
fn titulo(mundo: &Mundo, normal_maps: bool, rotando: bool) -> String {
    let si_no = |b: bool| if b { "si" } else { "no" };
    let hora = if mundo.horarios.len() > 1 {
        format!(" ({})", mundo.horario_actual())
    } else {
        String::new()
    };
    format!(
        "Raytracer - {}{hora} | flechas/mouse: rotar, W/S/rueda: zoom | R: girar ({}) | \
         T: hora | P: portal | N: mapas normales ({})",
        mundo.nombre,
        si_no(rotando),
        si_no(normal_maps)
    )
}
