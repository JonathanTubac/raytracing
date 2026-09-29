//! Skybox: el cielo que rodea la escena, guardado como un cubo de 6 imagenes (cubemap).
//!
//! Todo rayo que no choca con nada, incluidos los que salen de reflejos y refracciones,
//! toma su color del cubo segun su direccion. Las 6 caras se generan una sola vez al
//! arrancar con el cielo de Minecraft (degradado, sol y nubes cuadradas); despues, leer el
//! cielo es solo buscar un pixel en una imagen, sin recalcular nubes ni sol en cada rayo.

use std::thread;

use crate::color::Color;
use crate::math::{cross, dot, normalize, Vec3};
use crate::texture::ImageTexture;

// Cielo de dia de Minecraft en un bioma de llanura: azul arriba y mas claro en el horizonte
const CIELO_ARRIBA: Color = Color::new(120, 167, 255);
const CIELO_HORIZONTE: Color = Color::new(192, 216, 255);
// Debajo del horizonte (se ve porque el diorama flota) el cielo se oscurece
const CIELO_ABAJO: Color = Color::new(55, 80, 140);

// Mitad del lado del sol, medido en el plano a distancia 1 de la camara (como en el juego,
// donde el sol es un cuadrado de 30 bloques a 100 de distancia)
const SOL_MITAD: f32 = 0.2;
// El sol se suma al cielo; mas de 1 para que su centro brille y el tone mapping lo suavice
const SOL_BRILLO: f32 = 1.6;

// Las nubes son un plano a altura 1 sobre la camara. Cada pixel de clouds.png es un
// cuadrado de este tamano en ese plano, y la imagen se repite en ambas direcciones.
const NUBE_PIXEL: f32 = 0.08;
const NUBE_OPACIDAD: f32 = 0.8;
const NUBE_COLOR: Color = Color::new(250, 250, 255);

// Pixeles de lado de cada cara del cubo
const LADO_CARA: usize = 512;

/// Una cara del cubo: +X, -X, +Y, -Y, +Z, -Z, en ese orden
struct Face {
    pixels: Vec<Color>,
}

pub struct Skybox {
    size: usize,
    faces: Vec<Face>,
}

impl Skybox {
    /// Genera las 6 caras evaluando `sky` en la direccion de cada pixel. Cada cara se
    /// calcula en su propio hilo.
    pub fn from_fn(size: usize, sky: impl Fn(&Vec3) -> Color + Sync) -> Skybox {
        let sky = &sky;
        let faces = thread::scope(|scope| {
            let handles: Vec<_> = (0..6)
                .map(|face| {
                    scope.spawn(move || {
                        let mut pixels = Vec::with_capacity(size * size);
                        for y in 0..size {
                            for x in 0..size {
                                // Centro del pixel, de 0 a 1
                                let u = (x as f32 + 0.5) / size as f32;
                                let v = (y as f32 + 0.5) / size as f32;
                                pixels.push(sky(&normalize(&face_direction(face, u, v))));
                            }
                        }
                        Face { pixels }
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        Skybox { size, faces }
    }

    /// El cielo de dia de Minecraft con el sol en la direccion `sun_direction`
    pub fn minecraft(sun_direction: Vec3) -> Skybox {
        let sun = ImageTexture::load(&ruta("sun"));
        let clouds = ImageTexture::load(&ruta("clouds"));
        let sun_direction = normalize(&sun_direction);

        Skybox::from_fn(LADO_CARA, |direction| {
            let mut color = gradient(direction);
            color = with_clouds(color, direction, clouds);
            color + sun_color(direction, &sun_direction, sun)
        })
    }

    /// Color del cielo en esa direccion. Mezcla los 4 pixeles mas cercanos de la cara para
    /// que el degradado y los bordes del sol no se vean escalonados.
    pub fn sample(&self, direction: &Vec3) -> Color {
        let (face, u, v) = direction_to_face(direction);
        let pixels = &self.faces[face].pixels;
        let n = self.size;

        // Posicion en pixeles, contando desde el centro del primer pixel
        let x = (u * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
        let y = (v * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(n - 1), (y0 + 1).min(n - 1));
        let (tx, ty) = (x - x0 as f32, y - y0 as f32);

        let top = Color::lerp(pixels[y0 * n + x0], pixels[y0 * n + x1], tx);
        let bottom = Color::lerp(pixels[y1 * n + x0], pixels[y1 * n + x1], tx);
        Color::lerp(top, bottom, ty)
    }
}

fn ruta(nombre: &str) -> String {
    format!("{}/assets/textures/{nombre}.png", env!("CARGO_MANIFEST_DIR"))
}

/// Cara del cubo que ve la direccion y coordenadas (u, v) de 0 a 1 dentro de ella. La cara
/// es la del eje en que la direccion es mas larga; las otras dos componentes, divididas por
/// esa, dan la posicion dentro de la cara.
fn direction_to_face(d: &Vec3) -> (usize, f32, f32) {
    let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());

    let (face, major, sc, tc) = if ax >= ay && ax >= az {
        if d.x > 0.0 { (0, ax, -d.z, -d.y) } else { (1, ax, d.z, -d.y) }
    } else if ay >= az {
        if d.y > 0.0 { (2, ay, d.x, d.z) } else { (3, ay, d.x, -d.z) }
    } else if d.z > 0.0 {
        (4, az, d.x, -d.y)
    } else {
        (5, az, -d.x, -d.y)
    };

    (face, (sc / major + 1.0) * 0.5, (tc / major + 1.0) * 0.5)
}

/// Lo contrario de `direction_to_face`: la direccion (sin normalizar) del punto (u, v)
fn face_direction(face: usize, u: f32, v: f32) -> Vec3 {
    let (sc, tc) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    match face {
        0 => Vec3::new(1.0, -tc, -sc),
        1 => Vec3::new(-1.0, -tc, sc),
        2 => Vec3::new(sc, 1.0, tc),
        3 => Vec3::new(sc, -1.0, -tc),
        4 => Vec3::new(sc, -tc, 1.0),
        _ => Vec3::new(-sc, -tc, -1.0),
    }
}

/// Degradado vertical: azul arriba, claro en el horizonte y mas oscuro abajo
fn gradient(direction: &Vec3) -> Color {
    let y = direction.y;
    if y >= 0.0 {
        Color::lerp(CIELO_HORIZONTE, CIELO_ARRIBA, smoothstep(0.0, 0.45, y))
    } else {
        Color::lerp(CIELO_HORIZONTE, CIELO_ABAJO, smoothstep(0.0, 0.3, -y))
    }
}

/// Nubes: se proyecta la direccion sobre el plano de las nubes (altura 1) y se mira que
/// pixel de clouds.png cae ahi. Cerca del horizonte se desvanecen, como con la niebla del
/// juego, porque ahi los pixeles quedan tan lejos que se verian como ruido.
fn with_clouds(sky: Color, direction: &Vec3, clouds: &ImageTexture) -> Color {
    if direction.y <= 0.02 {
        return sky;
    }
    let distance = 1.0 / direction.y;
    let x = direction.x * distance / NUBE_PIXEL;
    let z = direction.z * distance / NUBE_PIXEL;

    // La imagen mide 256 pixeles y se repite
    let (_, alpha) = clouds.sample(x / 256.0, (z / 256.0).rem_euclid(1.0));
    let coverage = alpha.unwrap_or(1.0) * NUBE_OPACIDAD * smoothstep(0.03, 0.3, direction.y);
    Color::lerp(sky, NUBE_COLOR, coverage)
}

/// El sol es un cuadrado mirando a la camara. Se busca donde cae la direccion en el plano
/// del sol (a distancia 1, perpendicular a `sun_direction`) y, si cae dentro del cuadrado,
/// se toma ese pixel de sun.png. Su fondo negro no suma nada, asi el sol se funde con el
/// cielo como en el juego.
fn sun_color(direction: &Vec3, sun_direction: &Vec3, sun: &ImageTexture) -> Color {
    let facing = dot(direction, sun_direction);
    if facing <= 0.0 {
        return Color::black();
    }

    // Dos ejes perpendiculares al sol para ubicar el punto dentro del cuadrado. Se arman a
    // partir del eje vertical, salvo que el sol este justo arriba
    let helper = if sun_direction.y.abs() < 0.99 {
        Vec3::new(0.0, 1.0, 0.0)
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let right = normalize(&cross(sun_direction, &helper));
    let up = cross(&right, sun_direction);

    let on_plane = direction * (1.0 / facing);
    let x = dot(&on_plane, &right) / SOL_MITAD;
    let y = dot(&on_plane, &up) / SOL_MITAD;
    if x.abs() > 1.0 || y.abs() > 1.0 {
        return Color::black();
    }

    let (color, _) = sun.sample((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    color * SOL_BRILLO
}

/// 0 antes de `from`, 1 despues de `to`, y una curva suave en medio
fn smoothstep(from: f32, to: f32, x: f32) -> f32 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: Color, b: Color) -> bool {
        (a.r - b.r).abs() < 0.02 && (a.g - b.g).abs() < 0.02 && (a.b - b.b).abs() < 0.02
    }

    #[test]
    fn cada_direccion_vuelve_al_mismo_punto_de_su_cara() {
        for face in 0..6 {
            for (u, v) in [(0.1, 0.2), (0.5, 0.5), (0.9, 0.7), (0.3, 0.95)] {
                let direccion = face_direction(face, u, v);
                let (cara, u2, v2) = direction_to_face(&direccion);
                assert_eq!(cara, face);
                assert!((u - u2).abs() < 1e-5 && (v - v2).abs() < 1e-5, "cara {face}");
            }
        }
    }

    #[test]
    fn los_ejes_caen_en_el_centro_de_su_cara() {
        let ejes = [
            (Vec3::new(1.0, 0.0, 0.0), 0),
            (Vec3::new(-1.0, 0.0, 0.0), 1),
            (Vec3::new(0.0, 1.0, 0.0), 2),
            (Vec3::new(0.0, -1.0, 0.0), 3),
            (Vec3::new(0.0, 0.0, 1.0), 4),
            (Vec3::new(0.0, 0.0, -1.0), 5),
        ];
        for (eje, esperada) in ejes {
            assert_eq!(direction_to_face(&eje), (esperada, 0.5, 0.5));
        }
    }

    #[test]
    fn el_cubo_guarda_lo_que_calcula_la_funcion_del_cielo() {
        // Un cielo que depende de la direccion: rojo segun x, verde segun y, azul segun z
        let cielo = |d: &Vec3| Color::rgb(0.5 + d.x * 0.5, 0.5 + d.y * 0.5, 0.5 + d.z * 0.5);
        let skybox = Skybox::from_fn(64, cielo);

        for d in [
            Vec3::new(1.0, 0.2, -0.3),
            Vec3::new(-0.4, 1.0, 0.1),
            Vec3::new(0.3, -0.5, 1.0),
            Vec3::new(0.577, 0.577, -0.577),
        ] {
            let d = normalize(&d);
            assert!(cerca(skybox.sample(&d), cielo(&d)), "{d:?}");
        }
    }

    #[test]
    fn el_cielo_es_mas_azul_arriba_que_en_el_horizonte() {
        let arriba = gradient(&Vec3::new(0.0, 1.0, 0.0));
        let horizonte = gradient(&Vec3::new(1.0, 0.0, 0.0));
        assert!(arriba.r < horizonte.r && arriba.g < horizonte.g);
    }

    #[test]
    fn el_sol_se_ve_en_su_direccion_y_no_en_la_contraria() {
        let sun = ImageTexture::load(&ruta("sun"));
        let hacia_el_sol = normalize(&Vec3::new(-1.0, 1.0, 0.5));

        let centro = sun_color(&hacia_el_sol, &hacia_el_sol, sun);
        assert!(centro.r > 1.0, "el centro del sol brilla: {centro:?}");
        assert_eq!(sun_color(&-hacia_el_sol, &hacia_el_sol, sun), Color::black());
    }

    #[test]
    fn las_nubes_tapan_parte_del_cielo_y_no_bajan_del_horizonte() {
        let clouds = ImageTexture::load(&ruta("clouds"));
        let azul = Color::rgb(0.0, 0.0, 1.0);

        // Recorriendo el cielo, algunas direcciones tienen nube y otras no
        let con_nube = (0..200)
            .map(|i| normalize(&Vec3::new(i as f32 * 0.037, 1.0, i as f32 * 0.021)))
            .filter(|d| with_clouds(azul, d, clouds).r > 0.1)
            .count();
        assert!(con_nube > 10 && con_nube < 190, "{con_nube} de 200 con nube");

        assert_eq!(with_clouds(azul, &Vec3::new(1.0, -0.1, 0.0), clouds), azul);
    }
}
