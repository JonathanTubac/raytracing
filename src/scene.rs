//! Los dos mundos del diorama: la mina y el Nether al otro lado del portal. Cada uno tiene
//! sus bloques, la vista inicial de la camara, el lugar de su portal y uno o mas horarios
//! (la mina se puede ver de dia, al atardecer y de noche).

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

// Centro de la isla, alrededor del cual orbita la camara (igual en los dos mundos)
const CENTRO: Vec3 = Vec3::new(-0.5, -3.0, -0.5);

// El sol y la luna estan lejos para que sus rayos lleguen casi paralelos a toda la isla
const DISTANCIA_AL_CIELO: f32 = 150.0;

// Cuanto alumbran los bloques emisivos (por cada unidad de su `emission`) y hasta cuantos
// bloques de distancia
const LUZ_EMISIVA: f32 = 2.0;
const ALCANCE_EMISIVO: f32 = 8.0;

// Lado (en bloques) de los grupos en que se juntan los bloques emisivos para darles una
// sola luz
const GRUPO_LUCES: f32 = 3.0;

/// Como se ve un mundo a cierta hora: su cielo, sus luces y su luz ambiente
pub struct Horario {
    pub nombre: &'static str,
    pub skybox: Skybox,
    /// Todas las luces: las del cielo (sol o luna y el reflejo del cielo) y las de los
    /// bloques emisivos
    pub lights: Vec<Light>,
    /// Luz minima que recibe cualquier superficie
    pub ambient: Color,
}

pub struct Mundo {
    pub nombre: &'static str,
    pub world: VoxelWorld,
    pub horarios: Vec<Horario>,
    /// Cual de los horarios se esta viendo
    pub horario: usize,
    /// Vista con la que empieza la camara, y a la que vuelve despues de cruzar el portal
    pub home: Camera,
    /// Centro del portal de este mundo (la camara vuela a traves de el para cruzar)
    pub portal: Vec3,
}

impl Mundo {
    /// La escena que se le pasa al render, con el horario actual
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

    /// Pasa al horario siguiente (del ultimo vuelve al primero). Devuelve si cambio algo:
    /// un mundo con un solo horario se queda igual.
    pub fn siguiente_horario(&mut self) -> bool {
        let anterior = self.horario;
        self.horario = (self.horario + 1) % self.horarios.len();
        self.horario != anterior
    }
}

/// La vista inicial: al frente y un poco a la derecha, mirando el corte desde arriba
fn vista_inicial() -> Camera {
    let mut camera = Camera::new(CENTRO, 24.0);
    camera.orbit(0.3, 0.42);
    camera
}

/// Un horario de la mina: el cielo con su estilo, el sol (o la luna) justo donde se ve en
/// el cielo, asi las sombras caen del lado contrario, un reflejo del cielo desde el frente
/// para que el corte no quede negro, y las luces de los bloques emisivos
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

/// La mina, en tres horarios:
/// - Atardecer (el inicial): sol naranja, bajo y a la izquierda. La superficie queda dorada
///   y el interior de la mina a oscuras, alumbrado por la lava y la glowstone.
/// - Noche: la luna, fria y debil. Casi todo lo alumbran los bloques emisivos.
/// - Dia: sol blanco y alto, todo bien iluminado.
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

/// El Nether. No hay sol ni horas: lo alumbran la lava, la glowstone, el magma y la
/// shroomlight, mas un resplandor rojizo desde arriba (el techo del Nether reflejando la
/// lava) y otro mas debil desde el frente.
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

/// Una luz por cada grupo de bloques emisivos iguales y cercanos.
///
/// Poner una luz por bloque haria que un lago de lava de 60 bloques cueste como 60 luces
/// en cada punto de la escena. Los bloques se agrupan por material en cubos de
/// `GRUPO_LUCES` bloques de lado, y cada grupo es una sola luz en su centro que suma la
/// intensidad de todos: desde lejos ilumina igual, y cuesta mucho menos. Los bloques
/// enterrados (sin ninguna cara al aire) no alumbran nada y se ignoran.
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
            // La luz tiene el color promedio de la textura del bloque (naranja en la lava,
            // amarillo en la glowstone), llevado a que su canal mas fuerte valga 1
            let c = material.texture.average_color();
            let color = c * (1.0 / c.r.max(c.g).max(c.b).max(1e-3));
            let intensity = LUZ_EMISIVA * material.emission;
            Light::from_blocks(&centers, color, intensity, ALCANCE_EMISIVO)
        })
        .collect()
}
