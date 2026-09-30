//! Skybox: the sky around the scene, stored as a cube of 6 images (cubemap)

use std::thread;

use crate::color::Color;
use crate::math::{cross, dot, normalize, Vec3};
use crate::texture::ImageTexture;

/// Sky colors at a time of day
pub struct SkyStyle {
    /// Top, at the horizon and below the horizon (visible because the diorama floats)
    pub top: Color,
    pub horizon: Color,
    pub bottom: Color,
    /// Glow added to the horizon on the sun's side, like at sunset
    pub glow: Color,
    pub clouds: Color,
    /// How much the clouds cover what is behind them (0 = nothing, 1 = everything)
    pub cloud_opacity: f32,
    /// Color the sun (or moon) texture is multiplied by
    pub sun: Color,
    /// Whether the moon is in the sky instead of the sun
    pub moon: bool,
    /// Fraction of the sky with stars (0 = none)
    pub stars: f32,
}

/// Minecraft plains daytime sky: blue on top and lighter at the horizon
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

/// Sunset: orange horizon on the sun's side and pink clouds
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

/// Night: almost black blue, stars, the Minecraft moon and dark, sparse clouds
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

// Half the sun's side, on the plane at distance 1 from the camera
const SOL_MITAD: f32 = 0.2;
// The clouds are a plane at height 1 above the camera
const NUBE_PIXEL: f32 = 0.08;

// Texture rectangle (corner u, v, width and height) covering the whole image
const IMAGEN_ENTERA: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

// Pixels per side of each cube face
const LADO_CARA: usize = 512;

/// A cube face: +X, -X, +Y, -Y, +Z, -Z, in that order
struct Face {
    pixels: Vec<Color>,
}

pub struct Skybox {
    size: usize,
    faces: Vec<Face>,
}

impl Skybox {
    /// Generates the 6 faces by evaluating `sky` in each pixel's direction
    pub fn from_fn(size: usize, sky: impl Fn(&Vec3) -> Color + Sync) -> Skybox {
        let sky = &sky;
        let faces = thread::scope(|scope| {
            let handles: Vec<_> = (0..6)
                .map(|face| {
                    scope.spawn(move || {
                        let mut pixels = Vec::with_capacity(size * size);
                        for y in 0..size {
                            for x in 0..size {
                                // Pixel center, from 0 to 1
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

    /// Minecraft sky with the sun (or moon) at `sun_direction`
    pub fn minecraft(sun_direction: Vec3, style: &SkyStyle) -> Skybox {
        // The moon comes from moon_phases.png, a 4 x 2 grid of phases; the first is the full moon
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
            // The sun is added to the sky
            color + sun_color(direction, &sun_direction, body, rect) * style.sun
        })
    }

    /// Nether sky: red haze with ash
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

            // Ash specks in a few randomly chosen cells
            let cell = [d.x, d.y, d.z].map(|c| (c * 170.0).floor() as i32);
            if speck_hash(cell) < 0.005 { ash } else { fog }
        })
    }

    /// Sky color in that direction
    pub fn sample(&self, direction: &Vec3) -> Color {
        let (face, u, v) = direction_to_face(direction);
        let pixels = &self.faces[face].pixels;
        let n = self.size;

        // Position in pixels, counting from the center of the first pixel
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

/// Cube face the direction looks at, and (u, v) coordinates from 0 to 1 on it
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

/// Inverse of `direction_to_face`: the (unnormalized) direction of point (u, v)
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

/// Vertical gradient (top, horizon, bottom) plus the horizon glow on the sun's side
fn gradient(direction: &Vec3, sun_direction: &Vec3, style: &SkyStyle) -> Color {
    let y = direction.y;
    let base = if y >= 0.0 {
        Color::lerp(style.horizon, style.top, smoothstep(0.0, 0.45, y))
    } else {
        Color::lerp(style.horizon, style.bottom, smoothstep(0.0, 0.3, -y))
    };

    // The glow is strongest toward the sun and close to the horizon
    let flat = |v: &Vec3| normalize(&Vec3::new(v.x, 0.0, v.z));
    let toward_sun = dot(&flat(direction), &flat(sun_direction)).max(0.0);
    let near_horizon = 1.0 - smoothstep(0.0, 0.5, y.abs());
    base + style.glow * (toward_sun.powi(4) * near_horizon)
}

/// Stars in a few randomly chosen cells
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

/// Clouds: the clouds.png pixel on a plane at height 1
fn with_clouds(sky: Color, direction: &Vec3, clouds: &ImageTexture, style: &SkyStyle) -> Color {
    if direction.y <= 0.02 {
        return sky;
    }
    let distance = 1.0 / direction.y;
    let x = direction.x * distance / NUBE_PIXEL;
    let z = direction.z * distance / NUBE_PIXEL;

    // The image is 256 pixels wide and tiles
    let (_, alpha) = clouds.sample(x / 256.0, (z / 256.0).rem_euclid(1.0));
    let fade = smoothstep(0.03, 0.3, direction.y);
    let coverage = alpha.unwrap_or(1.0) * style.cloud_opacity * fade;
    Color::lerp(sky, style.clouds, coverage)
}

/// The sun is a square facing the camera
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

    // Two axes perpendicular to the sun to locate the point inside the square
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

/// Number between 0 and 1 that looks random but only depends on the cell
fn speck_hash([x, y, z]: [i32; 3]) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// 0 before `from`, 1 after `to`, and a smooth curve in between
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
        // A sky that depends on direction: red from x, green from y, blue from z
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

        // Across the sky, some directions have clouds and others don't
        let con_nube = (0..200)
            .map(|i| normalize(&Vec3::new(i as f32 * 0.037, 1.0, i as f32 * 0.021)))
            .filter(|d| with_clouds(azul, d, clouds, &DIA).r > 0.1)
            .count();
        assert!(con_nube > 10 && con_nube < 190, "{con_nube} de 200 con nube");

        let bajo_el_horizonte = Vec3::new(1.0, -0.1, 0.0);
        assert_eq!(with_clouds(azul, &bajo_el_horizonte, clouds, &DIA), azul);
    }
}
