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

/// Colores del cielo a una hora del dia
pub struct SkyStyle {
    /// Arriba, en el horizonte y debajo del horizonte (se ve porque el diorama flota)
    pub top: Color,
    pub horizon: Color,
    pub bottom: Color,
    /// Resplandor que se suma al horizonte del lado del sol, como al atardecer
    pub glow: Color,
    pub clouds: Color,
    /// Que tanto tapan las nubes lo que hay detras (0 = nada, 1 = todo)
    pub cloud_opacity: f32,
    /// Color por el que se multiplica la textura del sol (o de la luna)
    pub sun: Color,
    /// Si en el cielo esta la luna en vez del sol
    pub moon: bool,
    /// Que parte del cielo tiene estrellas (0 = ninguna)
    pub stars: f32,
}

/// Cielo de dia de Minecraft en un bioma de llanura: azul arriba y mas claro en el horizonte
pub const DIA: SkyStyle = SkyStyle {
    top: Color::new(120, 167, 255),
    horizon: Color::new(192, 216, 255),
    bottom: Color::new(55, 80, 140),
    glow: Color::black(),
    clouds: Color::new(250, 250, 255),
    cloud_opacity: 0.8,
    sun: Color::rgb(1.6, 1.6, 1.6),
    moon: false,
    stars: 0.0,
};

/// Atardecer: azul profundo arriba, horizonte naranja que se enciende del lado del sol, y
/// nubes rosadas
pub const ATARDECER: SkyStyle = SkyStyle {
    top: Color::new(52, 72, 140),
    horizon: Color::new(235, 150, 120),
    bottom: Color::new(40, 35, 70),
    glow: Color::rgb(0.9, 0.35, 0.05),
    clouds: Color::new(255, 190, 170),
    cloud_opacity: 0.8,
    sun: Color::rgb(2.0, 1.3, 0.7),
    moon: false,
    stars: 0.0,
};

/// Noche: azul casi negro, estrellas, la luna de Minecraft y nubes oscuras y ralas
pub const NOCHE: SkyStyle = SkyStyle {
    top: Color::new(4, 6, 18),
    horizon: Color::new(22, 28, 55),
    bottom: Color::new(4, 5, 14),
    glow: Color::black(),
    clouds: Color::new(40, 45, 65),
    cloud_opacity: 0.45,
    sun: Color::rgb(1.4, 1.45, 1.6),
    moon: true,
    stars: 0.012,
};

// Mitad del lado del sol, medido en el plano a distancia 1 de la camara (como en el juego,
// donde el sol es un cuadrado de 30 bloques a 100 de distancia)
const SOL_MITAD: f32 = 0.2;
// Las nubes son un plano a altura 1 sobre la camara. Cada pixel de clouds.png es un
// cuadrado de este tamano en ese plano, y la imagen se repite en ambas direcciones.
const NUBE_PIXEL: f32 = 0.08;

// Rectangulo de una textura (u, v de la esquina, ancho y alto) que abarca la imagen entera
const IMAGEN_ENTERA: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

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

    /// El cielo de Minecraft con los colores de `style` y el sol (o la luna) en la
    /// direccion `sun_direction`
    pub fn minecraft(sun_direction: Vec3, style: &SkyStyle) -> Skybox {
        // La luna sale de moon_phases.png, una grilla de 4 x 2 fases; la primera es la luna
        // llena
        let (body, rect) = if style.moon {
            (ImageTexture::load(&ruta("moon_phases")), [0.0, 0.0, 0.25, 0.5])
        } else {
            (ImageTexture::load(&ruta("sun")), IMAGEN_ENTERA)
        };
        let clouds = ImageTexture::load(&ruta("clouds"));
        let sun_direction = normalize(&sun_direction);

        Skybox::from_fn(LADO_CARA, |direction| {
            let mut color = gradient(direction, &sun_direction, style);
            color += stars(direction, style.stars);
            color = with_clouds(color, direction, clouds, style);
            // El sol se suma al cielo; puede pasar de 1 para que su centro brille y el tone
            // mapping lo suavice
            color + sun_color(direction, &sun_direction, body, rect) * style.sun
        })
    }

    /// El "cielo" del Nether: no hay sol ni nubes, solo una neblina roja mas densa hacia el
    /// horizonte, con motas de ceniza encendida flotando
    pub fn nether() -> Skybox {
        let top = Color::new(35, 6, 6);
        let horizon = Color::new(110, 28, 18);
        let bottom = Color::new(50, 8, 4);
        let ash = Color::rgb(1.4, 0.55, 0.2);

        Skybox::from_fn(LADO_CARA, |d| {
            let fog = if d.y >= 0.0 {
                Color::lerp(horizon, top, smoothstep(0.0, 0.6, d.y))
            } else {
                Color::lerp(horizon, bottom, smoothstep(0.0, 0.4, -d.y))
            };

            // Las motas son celdas chicas de una grilla sobre las direcciones; unas pocas,
            // elegidas al azar, brillan
            let cell = [d.x, d.y, d.z].map(|c| (c * 170.0).floor() as i32);
            if speck_hash(cell) < 0.005 { ash } else { fog }
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

/// Degradado vertical (arriba, horizonte, abajo) mas el resplandor del horizonte del lado
/// del sol
fn gradient(direction: &Vec3, sun_direction: &Vec3, style: &SkyStyle) -> Color {
    let y = direction.y;
    let base = if y >= 0.0 {
        Color::lerp(style.horizon, style.top, smoothstep(0.0, 0.45, y))
    } else {
        Color::lerp(style.horizon, style.bottom, smoothstep(0.0, 0.3, -y))
    };

    // El resplandor es mas fuerte mirando hacia el sol y pegado al horizonte
    let flat = |v: &Vec3| normalize(&Vec3::new(v.x, 0.0, v.z));
    let toward_sun = dot(&flat(direction), &flat(sun_direction)).max(0.0);
    let near_horizon = 1.0 - smoothstep(0.0, 0.5, y.abs());
    base + style.glow * (toward_sun.powi(4) * near_horizon)
}

/// Estrellas: celdas chicas de una grilla sobre las direcciones, unas pocas elegidas al
/// azar, cada una con su brillo. Se apagan cerca del horizonte, donde el aire es mas denso.
fn stars(direction: &Vec3, density: f32) -> Color {
    if density <= 0.0 || direction.y <= 0.0 {
        return Color::black();
    }
    let cell = [direction.x, direction.y, direction.z].map(|c| (c * 220.0).floor() as i32);
    if speck_hash(cell) >= density {
        return Color::black();
    }
    let brightness = 0.4 + 0.8 * speck_hash([cell[2], cell[0], cell[1]]);
    Color::rgb(1.0, 1.0, 1.1) * (brightness * smoothstep(0.0, 0.25, direction.y))
}

/// Nubes: se proyecta la direccion sobre el plano de las nubes (altura 1) y se mira que
/// pixel de clouds.png cae ahi. Cerca del horizonte se desvanecen, como con la niebla del
/// juego, porque ahi los pixeles quedan tan lejos que se verian como ruido.
fn with_clouds(sky: Color, direction: &Vec3, clouds: &ImageTexture, style: &SkyStyle) -> Color {
    if direction.y <= 0.02 {
        return sky;
    }
    let distance = 1.0 / direction.y;
    let x = direction.x * distance / NUBE_PIXEL;
    let z = direction.z * distance / NUBE_PIXEL;

    // La imagen mide 256 pixeles y se repite
    let (_, alpha) = clouds.sample(x / 256.0, (z / 256.0).rem_euclid(1.0));
    let fade = smoothstep(0.03, 0.3, direction.y);
    let coverage = alpha.unwrap_or(1.0) * style.cloud_opacity * fade;
    Color::lerp(sky, style.clouds, coverage)
}

/// El sol es un cuadrado mirando a la camara. Se busca donde cae la direccion en el plano
/// del sol (a distancia 1, perpendicular a `sun_direction`) y, si cae dentro del cuadrado,
/// se toma ese pixel de sun.png. Su fondo negro no suma nada, asi el sol se funde con el
/// cielo como en el juego. `rect` es la parte de la imagen que se usa (u, v de la esquina,
/// ancho y alto).
fn sun_color(
    direction: &Vec3,
    sun_direction: &Vec3,
    sun: &ImageTexture,
    rect: [f32; 4],
) -> Color {
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

    let [u0, v0, width, height] = rect;
    sun.sample(u0 + (x + 1.0) * 0.5 * width, v0 + (1.0 - y) * 0.5 * height).0
}

/// Numero entre 0 y 1 que parece al azar pero depende solo de la celda
fn speck_hash([x, y, z]: [i32; 3]) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
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
        let sol = normalize(&Vec3::new(-1.0, 1.0, 0.0));
        let arriba = gradient(&Vec3::new(0.0, 1.0, 0.0), &sol, &DIA);
        let horizonte = gradient(&Vec3::new(1.0, 0.0, 0.0), &sol, &DIA);
        assert!(arriba.r < horizonte.r && arriba.g < horizonte.g);
    }

    #[test]
    fn al_atardecer_el_horizonte_se_enciende_del_lado_del_sol() {
        let sol = normalize(&Vec3::new(-1.0, 0.3, 0.0));
        let hacia_el_sol = gradient(&Vec3::new(-1.0, 0.0, 0.0), &sol, &ATARDECER);
        let del_otro_lado = gradient(&Vec3::new(1.0, 0.0, 0.0), &sol, &ATARDECER);
        assert!(hacia_el_sol.r > del_otro_lado.r + 0.3);
    }

    #[test]
    fn el_sol_se_ve_en_su_direccion_y_no_en_la_contraria() {
        let sun = ImageTexture::load(&ruta("sun"));
        let hacia_el_sol = normalize(&Vec3::new(-1.0, 1.0, 0.5));

        let centro = sun_color(&hacia_el_sol, &hacia_el_sol, sun, IMAGEN_ENTERA);
        assert!(centro.r > 0.9, "el centro del sol brilla: {centro:?}");
        let del_otro_lado = sun_color(&-hacia_el_sol, &hacia_el_sol, sun, IMAGEN_ENTERA);
        assert_eq!(del_otro_lado, Color::black());
    }

    #[test]
    fn de_noche_hay_estrellas_arriba_y_no_bajo_el_horizonte() {
        let con_estrella = (0..2000)
            .map(|i| {
                let angulo = i as f32 * 0.37;
                normalize(&Vec3::new(angulo.sin(), 1.0, (angulo * 0.6).cos()))
            })
            .filter(|d| stars(d, NOCHE.stars).r > 0.0)
            .count();
        assert!(con_estrella > 0 && con_estrella < 200, "{con_estrella} de 2000");

        assert_eq!(stars(&Vec3::new(1.0, -0.2, 0.0), NOCHE.stars), Color::black());
        assert_eq!(stars(&Vec3::new(0.0, 1.0, 0.0), DIA.stars), Color::black());
    }

    #[test]
    fn las_nubes_tapan_parte_del_cielo_y_no_bajan_del_horizonte() {
        let clouds = ImageTexture::load(&ruta("clouds"));
        let azul = Color::rgb(0.0, 0.0, 1.0);

        // Recorriendo el cielo, algunas direcciones tienen nube y otras no
        let con_nube = (0..200)
            .map(|i| normalize(&Vec3::new(i as f32 * 0.037, 1.0, i as f32 * 0.021)))
            .filter(|d| with_clouds(azul, d, clouds, &DIA).r > 0.1)
            .count();
        assert!(con_nube > 10 && con_nube < 190, "{con_nube} de 200 con nube");

        let bajo_el_horizonte = Vec3::new(1.0, -0.1, 0.0);
        assert_eq!(with_clouds(azul, &bajo_el_horizonte, clouds, &DIA), azul);
    }
}
