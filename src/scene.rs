//! The two worlds of the diorama: the mine and the Nether across the portal

use crate::blocks::Blocks;
use crate::camera::Camera;
use crate::color::Color;
use crate::diorama::{mina, portal_mina};
use crate::light::Light;
use crate::math::{normalize, Vec3};
use crate::nether::{nether, portal_nether};
use crate::ray_intersect::Material;
use crate::render::Scene;
use crate::skybox::{Skybox, SkyStyle, ATARDECER, DIA, NOCHE};
use crate::voxel::VoxelWorld;

// Island center the camera orbits around (same in both worlds)
const CENTRO: Vec3 = Vec3::new(-0.5, -3.0, -0.5);

// The sun and moon are far away so their rays reach the island almost parallel
const DISTANCIA_AL_CIELO: f32 = 150.0;

// Intensity and range of emissive block light
const LUZ_EMISIVA: f32 = 2.0;
const ALCANCE_EMISIVO: f32 = 8.0;

// Side (in blocks) of the groups emissive blocks are merged into, one light each
const GRUPO_LUCES: f32 = 3.0;

/// How a world looks at a given time: its sky, lights and ambient light
pub struct Horario {
    pub nombre: &'static str,
    pub skybox: Skybox,
    /// Sky lights and emissive block lights
    pub lights: Vec<Light>,
    /// Minimum light any surface receives
    pub ambient: Color,
}

pub struct Mundo {
    pub nombre: &'static str,
    pub world: VoxelWorld,
    pub horarios: Vec<Horario>,
    /// Which time of day is being shown
    pub horario: usize,
    /// View the camera starts with, and returns to after going through the portal
    pub home: Camera,
    /// Center of this world's portal (the camera flies through it to cross)
    pub portal: Vec3,
}

impl Mundo {
    /// Scene passed to the renderer, with the current time of day
    pub fn scene(&self, normal_maps: bool) -> Scene<'_, VoxelWorld> {
        let horario = &self.horarios[self.horario];
        Scene {
            objects: std::slice::from_ref(&self.world),
            lights: &horario.lights,
            skybox: &horario.skybox,
            ambient: horario.ambient,
            normal_maps,
        }
    }

    pub fn horario_actual(&self) -> &'static str {
        self.horarios[self.horario].nombre
    }

    /// Moves to the next time of day (wrapping from the last to the first)
    pub fn siguiente_horario(&mut self) -> bool {
        let anterior = self.horario;
        self.horario = (self.horario + 1) % self.horarios.len();
        self.horario != anterior
    }
}

/// Initial view: in front and slightly to the right, looking down at the cut
fn vista_inicial() -> Camera {
    let mut camera = Camera::new(CENTRO, 24.0);
    camera.orbit(0.3, 0.42);
    camera
}

/// A mine time of day: sky, sun or moon, sky bounce light and emissive lights
fn horario_mina(
    nombre: &'static str,
    style: &SkyStyle,
    hacia_el_cielo: Vec3,
    luz: (Color, f32),
    reflejo: (Color, f32),
    ambient: Color,
    emisivas: &[Light],
) -> Horario {
    let astro = CENTRO + normalize(&hacia_el_cielo) * DISTANCIA_AL_CIELO;
    let mut lights = vec![
        Light::new(astro, luz.0, luz.1),
        Light::new(CENTRO + Vec3::new(40.0, 30.0, 60.0), reflejo.0, reflejo.1),
    ];
    lights.extend_from_slice(emisivas);

    Horario {
        nombre,
        skybox: Skybox::minecraft(hacia_el_cielo, style),
        lights,
        ambient,
    }
}

/// The mine at three times of day: sunset (the initial one), night and day
pub fn superficie(b: &Blocks) -> Mundo {
    let world = VoxelWorld::new(&mina(b));
    let emisivas = luces_emisivas(&world);

    let horarios = vec![
        horario_mina(
            "atardecer",
            &ATARDECER,
            Vec3::new(-0.9, 0.45, -0.4),
            (Color::rgb(1.0, 0.62, 0.35), 1.3),
            (Color::rgb(0.55, 0.55, 0.75), 0.6),
            Color::rgb(0.15, 0.14, 0.18),
            &emisivas,
        ),
        horario_mina(
            "noche",
            &NOCHE,
            Vec3::new(0.55, 0.7, -0.45),
            (Color::rgb(0.6, 0.7, 1.0), 0.35),
            (Color::rgb(0.3, 0.35, 0.6), 0.2),
            Color::rgb(0.05, 0.06, 0.1),
            &emisivas,
        ),
        horario_mina(
            "dia",
            &DIA,
            Vec3::new(-0.45, 0.85, -0.3),
            (Color::rgb(1.0, 0.97, 0.9), 1.0),
            (Color::rgb(0.6, 0.7, 1.0), 0.45),
            Color::rgb(0.22, 0.24, 0.28),
            &emisivas,
        ),
    ];

    Mundo {
        nombre: "Mina",
        world,
        horarios,
        horario: 0,
        home: vista_inicial(),
        portal: portal_mina(),
    }
}

/// The Nether
pub fn inframundo(b: &Blocks) -> Mundo {
    let world = VoxelWorld::new(&nether(b));

    let mut lights = vec![
        Light::new(CENTRO + Vec3::new(-20.0, 80.0, 30.0), Color::rgb(1.0, 0.45, 0.3), 0.6),
        Light::new(CENTRO + Vec3::new(40.0, 20.0, 60.0), Color::rgb(0.8, 0.45, 0.4), 0.45),
    ];
    lights.extend(luces_emisivas(&world));

    Mundo {
        nombre: "Nether",
        world,
        horarios: vec![Horario {
            nombre: "siempre",
            skybox: Skybox::nether(),
            lights,
            ambient: Color::rgb(0.22, 0.12, 0.1),
        }],
        horario: 0,
        home: vista_inicial(),
        portal: portal_nether(),
    }
}

/// One light per group of identical nearby emissive blocks
fn luces_emisivas(world: &VoxelWorld) -> Vec<Light> {
    let mut groups: Vec<([i32; 3], Material, Vec<Vec3>)> = Vec::new();
    for (center, material) in world.emissive_blocks() {
        let key = [0, 1, 2].map(|axis| (center[axis] / GRUPO_LUCES).floor() as i32);
        match groups.iter_mut().find(|(k, m, _)| *k == key && *m == material) {
            Some((_, _, centers)) => centers.push(center),
            None => groups.push((key, material, vec![center])),
        }
    }

    groups
        .into_iter()
        .map(|(_, material, centers)| {
            // Average texture color, with its strongest channel at 1
            let c = material.texture.average_color();
            let color = c * (1.0 / c.r.max(c.g).max(c.b).max(1e-3));
            let intensity = LUZ_EMISIVA * material.emission;
            Light::from_blocks(&centers, color, intensity, ALCANCE_EMISIVO)
        })
        .collect()
}
