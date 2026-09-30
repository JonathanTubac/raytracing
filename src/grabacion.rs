//! Diorama video recording: a scripted tour that shows each effect with a caption

use std::f32::consts::TAU;
use std::io::{self, Write};

use crate::camera::Camera;
use crate::framebuffer::Framebuffer;
use crate::math::Vec3;
use crate::render::render;
use crate::scene::Mundo;
use crate::texto;
use crate::viaje::{Paso, Viaje};

pub const ANCHO: usize = 1280;
pub const ALTO: usize = 720;
pub const FPS: f32 = 30.0;

// Indices of the worlds and of the mine times of day (see scene.rs)
const MINA: usize = 0;
const NETHER: usize = 1;
const ATARDECER: usize = 0;
const NOCHE: usize = 1;
const DIA: usize = 2;

/// Camera looking at `center` from `distance`, turned by `yaw` and raised by `pitch`
fn vista(center: Vec3, distance: f32, yaw: f32, pitch: f32) -> Camera {
    let mut camera = Camera::new(center, distance);
    camera.yaw = yaw;
    camera.pitch = pitch;
    camera
}

/// Smooth curve from 0 to 1 (starts and stops slowly)
fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Counts the frames and writes them out
struct Grabador<'a, W: Write> {
    fb: Framebuffer,
    out: &'a mut W,
    cuadros: usize,
}

impl<W: Write> Grabador<'_, W> {
    /// Renders a frame of the world with that camera, adds the caption and writes it
    fn cuadro(
        &mut self,
        mundo: &Mundo,
        camera: &Camera,
        normal_maps: bool,
        subtitulo: &str,
        overlay: f32,
    ) -> io::Result<()> {
        render(&mut self.fb, &mundo.scene(normal_maps), camera, 1);
        self.fb.portal_swirl(overlay, self.cuadros as f32 / FPS);

        if !subtitulo.is_empty() {
            let escala = 4;
            let x = (ANCHO - texto::ancho(subtitulo, escala)) / 2;
            texto::escribir(&mut self.fb, x, ALTO - 70, escala, subtitulo, 0xFFFFFF);
        }

        self.out.write_all(&self.fb.to_rgb24())?;
        self.cuadros += 1;
        if self.cuadros % 30 == 0 {
            eprint!("\rgrabando: {} cuadros ({:.0} s)", self.cuadros, self.cuadros as f32 / FPS);
        }
        Ok(())
    }

    /// A shot lasting `segundos` seconds: `camara(t)` gives the camera for t from 0 to 1
    fn toma(
        &mut self,
        mundo: &Mundo,
        segundos: f32,
        normal_maps: bool,
        subtitulo: &str,
        camara: impl Fn(f32) -> Camera,
    ) -> io::Result<()> {
        let cuadros = (segundos * FPS) as usize;
        for i in 0..cuadros {
            let t = i as f32 / (cuadros - 1).max(1) as f32;
            self.cuadro(mundo, &camara(t), normal_maps, subtitulo, 0.0)?;
        }
        Ok(())
    }
}

/// Records the whole tour. Leaves the worlds as it found them.
pub fn grabar(mundos: &mut [Mundo; 2], out: &mut impl Write) -> io::Result<()> {
    let mut g = Grabador {
        fb: Framebuffer::new(ANCHO, ALTO),
        out,
        cuadros: 0,
    };
    let home = mundos[MINA].home;
    let centro = home.center;
    let (yaw0, pitch0) = (home.yaw, home.pitch);

    // 1. The mine at sunset: half a turn around the island
    mundos[MINA].horario = ATARDECER;
    let mina = &mundos[MINA];
    g.toma(mina, 7.0, true, "MINA EN RUST - RAYTRACING SIN LIBRERIAS", |t| {
        vista(centro, 19.0, yaw0 + 1.6 * ease(t), pitch0)
    })?;

    // 2. The cut: lava, glowstone, gold and diamond, water
    let corte = Vec3::new(1.5, -6.0, 6.0);
    g.toma(mina, 5.0, true, "EMISIVOS: LAVA Y GLOWSTONE", |t| {
        vista(corte, 11.0 - 2.0 * t, -0.35 + 0.5 * ease(t), 0.12 + 0.1 * t)
    })?;
    g.toma(mina, 4.0, true, "REFLEXION: ORO, DIAMANTE Y OBSIDIANA", |t| {
        vista(Vec3::new(1.5, -7.0, 6.0), 6.0 - 1.0 * t, 0.15 + 0.25 * ease(t), 0.18)
    })?;

    // 3. Normal maps: the same view of the cut without and with relief
    let pared = |t: f32| vista(Vec3::new(-3.0, -5.0, 8.0), 6.5, -0.45 + 0.2 * t, 0.1);
    g.toma(mina, 2.5, false, "MAPAS NORMALES: NO", |t| pared(t * 0.5))?;
    g.toma(mina, 2.5, true, "MAPAS NORMALES: SI", |t| pared(0.5 + t * 0.5))?;

    // 4. The pond and the glass: refraction and sky reflection
    g.toma(mina, 5.0, true, "REFRACCION: AGUA Y VIDRIO", |t| {
        vista(Vec3::new(5.5, 0.0, 5.0), 7.0 - 1.5 * t, 0.2 + 0.7 * ease(t), 0.55)
    })?;

    // 5. Night and day
    mundos[MINA].horario = NOCHE;
    g.toma(&mundos[MINA], 5.0, true, "NOCHE: LA LUNA Y LOS BLOQUES EMISIVOS", |t| {
        vista(centro, 19.0, yaw0 - 0.6 + 1.2 * ease(t), pitch0 + 0.05)
    })?;
    mundos[MINA].horario = DIA;
    g.toma(&mundos[MINA], 4.0, true, "DIA", |t| {
        vista(centro, 20.0, yaw0 + 1.0 - 0.8 * ease(t), pitch0 + 0.1)
    })?;

    // 6. Back to sunset and through the hut portal to the Nether
    mundos[MINA].horario = ATARDECER;
    g.toma(&mundos[MINA], 2.0, true, "PORTAL AL NETHER", |t| {
        vista(centro, 20.0 + 4.0 * t, yaw0 - 0.3 * t, pitch0)
    })?;
    let mut camera = vista(centro, 24.0, yaw0 - 0.3, pitch0);
    let mut viaje = Viaje::new(&camera);
    let mut actual = MINA;
    loop {
        let mundo = &mundos[actual];
        match viaje.avanzar(1.0 / FPS, &mut camera, mundo.portal, &mundo.home) {
            Paso::CambiarMundo => actual = NETHER,
            Paso::Termino => break,
            Paso::Sigue => {}
        }
        let subtitulo = if actual == MINA { "PORTAL AL NETHER" } else { "EL NETHER" };
        g.cuadro(&mundos[actual], &camera, true, subtitulo, viaje.overlay())?;
    }

    // 7. The Nether: a turn around it and a closer look at the bridge over the lava
    let nether = &mundos[NETHER];
    let (nyaw, npitch) = (nether.home.yaw, nether.home.pitch);
    g.toma(nether, 7.0, true, "EL NETHER", |t| {
        vista(centro, 24.0 - 5.0 * ease(t), nyaw + TAU * 0.4 * ease(t), npitch - 0.05 * t)
    })?;
    g.toma(nether, 5.0, true, "LAGO DE LAVA Y FORTALEZA", |t| {
        vista(Vec3::new(4.0, -2.0, 5.0), 13.0 - 3.0 * t, 0.9 - 0.7 * ease(t), 0.3)
    })?;

    // 8. Ending: the initial Nether view
    g.toma(nether, 3.0, true, "", |t| {
        vista(centro, 18.0 + 3.0 * ease(t), nyaw + TAU * 0.4 + 0.3 * t, npitch)
    })?;

    eprintln!("\rgrabacion terminada: {} cuadros ({:.1} s)", g.cuadros, g.cuadros as f32 / FPS);
    g.out.flush()
}
