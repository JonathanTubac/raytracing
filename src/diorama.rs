//! El diorama: una isla flotante de 20 x 20 bloques con una mina adentro, cortada por el
//! frente como las maquetas de Minecraft para que se vea el interior.
//!
//! - Superficie: pasto, una colina, dos arboles, la entrada de la mina con su marco de
//!   madera y la cabana del minero, con un portal al Nether en la pared del fondo. Un
//!   estanque con orilla de arena refleja el cielo, y sobre la caverna hay un tragaluz de
//!   vidrio por donde entra el sol, refractado.
//! - Un tunel en escalera baja pegado al frente, con soportes de madera como un mineshaft.
//! - La caverna tiene glowstone colgando del techo, una poza de lava, un estanque de agua,
//!   obsidiana donde se juntan (la lava que toca agua se vuelve obsidiana), un alijo de oro
//!   y diamante, y una geoda de amatista en la pared.
//! - Las paredes son piedra y, en lo profundo, deepslate, con minerales segun la
//!   profundidad como en el juego (carbon arriba, diamante abajo). La base es bedrock.
//!
//! La forma de la caverna y la ubicacion de los minerales salen de un ruido propio, asi que
//! la mina es siempre la misma pero no se ve hecha a mano.

use crate::blocks::Blocks;
use crate::math::Vec3;
use crate::ray_intersect::Material;

// Extension de la isla (inclusive). El frente, donde esta el corte, es z = MAX_Z.
pub(crate) const MIN_X: i32 = -10;
pub(crate) const MAX_X: i32 = 9;
pub(crate) const MIN_Z: i32 = -10;
pub(crate) const MAX_Z: i32 = 9;
pub(crate) const MIN_Y: i32 = -12;
pub(crate) const MAX_Y: i32 = 10;

// Altura de la superficie (el pasto) fuera de la colina
pub(crate) const SUPERFICIE: i32 = 0;
// Desde esta altura hacia abajo la piedra es deepslate
const DEEPSLATE: i32 = -8;
// Altura del agua y la lava en el fondo de la caverna
const NIVEL_LIQUIDO: i32 = -8;

/// Los bloques de la isla en una matriz 3D mientras se construye
pub(crate) struct Grid {
    cells: Vec<Option<Material>>,
}

impl Grid {
    pub(crate) fn new() -> Grid {
        let size = ((MAX_X - MIN_X + 1) * (MAX_Y - MIN_Y + 1) * (MAX_Z - MIN_Z + 1)) as usize;
        Grid {
            cells: vec![None; size],
        }
    }

    fn index(x: i32, y: i32, z: i32) -> Option<usize> {
        let inside = (MIN_X..=MAX_X).contains(&x)
            && (MIN_Y..=MAX_Y).contains(&y)
            && (MIN_Z..=MAX_Z).contains(&z);
        inside.then(|| {
            let (w, h) = (MAX_X - MIN_X + 1, MAX_Y - MIN_Y + 1);
            (((z - MIN_Z) * h + (y - MIN_Y)) * w + (x - MIN_X)) as usize
        })
    }

    pub(crate) fn get(&self, x: i32, y: i32, z: i32) -> Option<Material> {
        Grid::index(x, y, z).and_then(|i| self.cells[i])
    }

    pub(crate) fn set(&mut self, x: i32, y: i32, z: i32, block: Option<Material>) {
        if let Some(i) = Grid::index(x, y, z) {
            self.cells[i] = block;
        }
    }

    pub(crate) fn is_air(&self, x: i32, y: i32, z: i32) -> bool {
        self.get(x, y, z).is_none()
    }

    /// Si alguna de las 6 celdas vecinas es aire
    pub(crate) fn touches_air(&self, x: i32, y: i32, z: i32) -> bool {
        [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
            .iter()
            .any(|(dx, dy, dz)| self.is_air(x + dx, y + dy, z + dz))
    }

    pub(crate) fn into_blocks(self) -> Vec<([i32; 3], Material)> {
        let mut blocks = Vec::new();
        for z in MIN_Z..=MAX_Z {
            for y in MIN_Y..=MAX_Y {
                for x in MIN_X..=MAX_X {
                    if let Some(material) = self.get(x, y, z) {
                        blocks.push(([x, y, z], material));
                    }
                }
            }
        }
        blocks
    }
}

/// Numero entre 0 y 1 que parece al azar pero depende solo de la celda y la semilla, asi
/// la mina sale igual cada vez
pub(crate) fn hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F)
        ^ seed.wrapping_mul(0x9E37_79B9);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Ruido suave: interpola el `hash` de las 8 esquinas enteras alrededor del punto, asi
/// puntos cercanos dan valores parecidos (sirve para deformar la caverna sin que quede
/// llena de agujeros sueltos)
pub(crate) fn noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (x0, y0, z0) = (x.floor(), y.floor(), z.floor());
    let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
    let (tx, ty, tz) = (smooth(x - x0), smooth(y - y0), smooth(z - z0));
    let (x0, y0, z0) = (x0 as i32, y0 as i32, z0 as i32);
    let h = |dx, dy, dz| hash(x0 + dx, y0 + dy, z0 + dz, seed);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;

    let bottom = lerp(
        lerp(h(0, 0, 0), h(1, 0, 0), tx),
        lerp(h(0, 0, 1), h(1, 0, 1), tx),
        tz,
    );
    let top = lerp(
        lerp(h(0, 1, 0), h(1, 1, 0), tx),
        lerp(h(0, 1, 1), h(1, 1, 1), tx),
        tz,
    );
    lerp(bottom, top, ty)
}

/// Altura del pasto en la columna (x, z): plano, salvo una colina atras a la izquierda,
/// deformada con ruido para que no quede como una piramide perfecta
fn surface_height(x: i32, z: i32) -> i32 {
    let (dx, dz) = (x as f32 + 5.0, z as f32 + 6.0);
    let distance = (dx * dx + dz * dz).sqrt();
    let bumps = (noise(x as f32 * 0.4, 0.0, z as f32 * 0.4, 8) - 0.5) * 2.5;
    let hill = 4.5 * (1.0 - (distance / 7.0).powi(2)) + bumps;
    SUPERFICIE + hill.max(0.0).round() as i32
}

/// Si la celda es parte de la caverna: dos elipsoides (la camara principal y una cuenca
/// mas baja para el estanque) deformados con ruido para que las paredes sean irregulares
fn is_cave(x: i32, y: i32, z: i32) -> bool {
    if !(-10..=-3).contains(&y) {
        return false;
    }
    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
    let ellipsoid = |cx: f32, cy: f32, cz: f32, rx: f32, ry: f32, rz: f32| {
        ((fx - cx) / rx).powi(2) + ((fy - cy) / ry).powi(2) + ((fz - cz) / rz).powi(2)
    };
    let chamber = ellipsoid(1.5, -5.5, 6.5, 7.5, 3.4, 5.0);
    let basin = ellipsoid(5.0, -8.0, 6.5, 4.5, 1.8, 3.5);
    let wobble = (noise(fx * 0.35, fy * 0.35, fz * 0.35, 7) - 0.5) * 0.7;
    chamber.min(basin) + wobble < 1.0
}

/// Arma la mina completa: la lista de bloques con su posicion y material
pub fn mina(b: &Blocks) -> Vec<([i32; 3], Material)> {
    let mut grid = Grid::new();

    terrain(&mut grid, b);
    ores(&mut grid, b);
    cave(&mut grid, b);
    tunnel(&mut grid, b);
    pond(&mut grid, b);
    skylight(&mut grid, b);
    hut(&mut grid, b);
    tree(&mut grid, b, 5, -5);
    tree(&mut grid, b, 1, -8);
    treasure(&mut grid, b);

    grid.into_blocks()
}

/// Capas de la isla: bedrock abajo (irregular en la segunda capa), deepslate, piedra, tres
/// capas de tierra y el pasto arriba
fn terrain(grid: &mut Grid, b: &Blocks) {
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            let height = surface_height(x, z);
            for y in MIN_Y..=height {
                let block = if y == MIN_Y || (y == MIN_Y + 1 && hash(x, y, z, 1) < 0.5) {
                    b.bedrock
                } else if y <= DEEPSLATE {
                    b.deepslate
                } else if y < height - 3 {
                    b.stone
                } else if y < height {
                    b.dirt
                } else {
                    b.grass
                };
                grid.set(x, y, z, Some(block));
            }
        }
    }
}

/// Minerales en la piedra segun la profundidad, como en el juego: carbon cerca de la
/// superficie, hierro en el medio, y oro y diamante abajo
fn ores(grid: &mut Grid, b: &Blocks) {
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            for y in MIN_Y..=SUPERFICIE {
                let current = grid.get(x, y, z);
                // Vetas de 2 o 3 bloques: el ruido de baja frecuencia agrupa los minerales
                let clump = noise(x as f32 * 0.5, y as f32 * 0.5, z as f32 * 0.5, 3);
                let r = hash(x, y, z, 2) * 0.6 + clump * 0.4;

                let ore = if current == Some(b.stone) {
                    if y > -5 && r < 0.12 {
                        Some(b.coal_ore)
                    } else if y <= -3 && r > 0.9 {
                        Some(b.iron_ore)
                    } else if y <= -5 && (0.46..0.5).contains(&r) {
                        Some(b.gold_ore)
                    } else {
                        None
                    }
                } else if current == Some(b.deepslate) {
                    if r < 0.08 {
                        Some(b.deepslate_diamond_ore)
                    } else if r > 0.92 {
                        Some(b.deepslate_gold_ore)
                    } else {
                        None
                    }
                } else {
                    None
                };
                if ore.is_some() {
                    grid.set(x, y, z, ore);
                }
            }
        }
    }
}

/// Excava la caverna y la amuebla: lava y agua en el fondo, obsidiana donde se juntan,
/// grava bajo el agua, glowstone en el techo y una geoda de amatista en la pared
fn cave(grid: &mut Grid, b: &Blocks) {
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            for y in MIN_Y..=SUPERFICIE {
                if is_cave(x, y, z) {
                    grid.set(x, y, z, None);
                }
            }
        }
    }

    // Fondo: lava a la izquierda, agua a la derecha, y entre las dos una franja de
    // obsidiana (la lava que toca agua se enfria y se vuelve obsidiana)
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            for y in MIN_Y..=NIVEL_LIQUIDO {
                if !is_cave(x, y, z) {
                    continue;
                }
                let block = if x >= 3 {
                    b.water
                } else if x <= 0 {
                    b.lava
                } else {
                    b.obsidian
                };
                grid.set(x, y, z, Some(block));
            }
        }
    }

    // Bajo el agua el fondo es grava
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            for y in MIN_Y + 1..=NIVEL_LIQUIDO {
                if grid.get(x, y, z) == Some(b.water) && grid.get(x, y - 1, z) != Some(b.water) {
                    grid.set(x, y - 1, z, Some(b.gravel));
                }
            }
        }
    }

    // Glowstone colgando del techo: bloques del techo elegidos al azar, con uno mas
    // colgando debajo de algunos para que se vean como racimos
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            for y in -8..=-3 {
                let ceiling = !grid.is_air(x, y + 1, z) && grid.is_air(x, y, z);
                if ceiling && is_cave(x, y, z) && hash(x, y, z, 4) < 0.22 {
                    grid.set(x, y + 1, z, Some(b.glowstone));
                    if hash(x, y, z, 5) < 0.4 && grid.is_air(x, y - 1, z) {
                        grid.set(x, y, z, Some(b.glowstone));
                    }
                }
            }
        }
    }

    // Geoda: la pared de la caverna se vuelve amatista alrededor de un punto
    let (gx, gy, gz) = (7.0, -5.0, 2.5);
    for x in 4..=MAX_X {
        for y in -8..=-2 {
            for z in 0..=5 {
                let (dx, dy, dz) = (x as f32 - gx, y as f32 - gy, z as f32 - gz);
                let d = (dx * dx + dy * dy + dz * dz).sqrt();
                if d < 2.6 && !grid.is_air(x, y, z) && grid.touches_air(x, y, z) {
                    grid.set(x, y, z, Some(b.amethyst));
                }
            }
        }
    }
}

/// Tunel en escalera pegado al frente de la isla, desde la superficie a la izquierda hasta
/// la caverna, con piso de tablones y un marco de madera (dos postes y una viga) cada dos
/// escalones, como los mineshafts del juego
fn tunnel(grid: &mut Grid, b: &Blocks) {
    let width = MAX_Z - 2..=MAX_Z;
    for step in 0..=7 {
        let x = MIN_X + 1 + step;
        let floor = SUPERFICIE - 1 - step;

        for z in width.clone() {
            grid.set(x, floor, z, Some(b.planks));
            for y in floor + 1..=floor + 3 {
                grid.set(x, y, z, None);
            }
        }

        if step % 2 == 1 {
            // Poste atras (el frente queda libre para ver adentro) y viga arriba
            for y in floor + 1..=floor + 2 {
                grid.set(x, y, MAX_Z - 2, Some(b.log));
            }
            for z in width.clone() {
                grid.set(x, floor + 3, z, Some(b.planks));
            }
        }
    }

    // Marco de la entrada en la superficie, con una glowstone arriba que la ilumina
    let x = MIN_X;
    let base = surface_height(x, MAX_Z);
    for y in base + 1..=base + 2 {
        grid.set(x, y, MAX_Z - 2, Some(b.log));
    }
    for z in width {
        grid.set(x, base + 3, z, Some(b.planks));
    }
    grid.set(x, base + 4, MAX_Z - 1, Some(b.glowstone));
}

/// Estanque en la superficie, adelante a la derecha: agua de un bloque de hondo con arena
/// en el fondo y en la orilla, y una linterna marina que lo ilumina
fn pond(grid: &mut Grid, b: &Blocks) {
    let (x0, x1, z0, z1) = (5, 8, 4, 7);
    for x in x0 - 1..=x1 + 1 {
        for z in z0 - 1..=z1 + 1 {
            let y = surface_height(x, z);
            if (x0..=x1).contains(&x) && (z0..=z1).contains(&z) {
                grid.set(x, y, z, Some(b.water));
                grid.set(x, y - 1, z, Some(b.sand));
            } else {
                grid.set(x, y, z, Some(b.sand));
            }
        }
    }

    // Una linterna marina en el fondo ilumina el agua desde abajo
    grid.set(x0 + 1, surface_height(x0 + 1, z0 + 2) - 1, z0 + 2, Some(b.sea_lantern));
}

// Cabana del minero: esquinas (inclusive) y altura de las paredes
const CABANA: (i32, i32, i32, i32) = (-10, -4, -1, 4);
const CABANA_ALTO: i32 = 5;

/// Centro del portal al Nether que esta en la pared del fondo de la cabana. La camara
/// vuela hasta aca para pasar al otro mundo.
pub fn portal_mina() -> Vec3 {
    // El portal ocupa las dos columnas del medio de la pared: su centro queda entre ellas
    let (x0, x1, z0, _) = CABANA;
    Vec3::new(((x0 + x1) / 2) as f32 + 0.5, SUPERFICIE as f32 + 2.0, z0 as f32)
}

/// Cabana del minero: piso de piedra, paredes de tablones con esquinas de tronco, ventanas
/// de vidrio a los lados, techo escalonado y una glowstone adentro. En la pared del fondo
/// esta el portal al Nether, y la puerta del frente queda justo delante, asi desde afuera
/// se ve el portal brillando por la puerta.
fn hut(grid: &mut Grid, b: &Blocks) {
    let (x0, x1, z0, z1) = CABANA;
    let floor = SUPERFICIE;
    let top = floor + CABANA_ALTO;
    let middle_x = (x0 + x1) / 2;

    for x in x0..=x1 {
        for z in z0..=z1 {
            // Cimiento: todo lleno hasta el piso, y aire arriba (por si la colina llegaba ahi)
            for y in floor - 2..floor {
                if grid.is_air(x, y, z) {
                    grid.set(x, y, z, Some(b.dirt));
                }
            }
            grid.set(x, floor, z, Some(b.cobblestone));
            for y in floor + 1..=top + 3 {
                grid.set(x, y, z, None);
            }

            // Paredes: troncos en las esquinas y tablones en el resto
            let edge_x = x == x0 || x == x1;
            let edge_z = z == z0 || z == z1;
            if edge_x || edge_z {
                let wall = if edge_x && edge_z { b.log } else { b.planks };
                for y in floor + 1..=top {
                    grid.set(x, y, z, Some(wall));
                }
            }

            // Techo escalonado: una capa entera, otra un bloque mas adentro y una cumbrera
            grid.set(x, top + 1, z, Some(b.planks));
            if (x0 + 1..x1).contains(&x) && (z0 + 1..z1).contains(&z) {
                grid.set(x, top + 2, z, Some(b.planks));
            }
            if (x0 + 2..x1 - 1).contains(&x) && (z0 + 2..z1 - 1).contains(&z) {
                grid.set(x, top + 3, z, Some(b.log));
            }
        }
    }

    // Portal en la pared del fondo: marco de obsidiana de 4 x 5 (el minimo del juego) con
    // el portal de 2 x 3 adentro
    let (px0, px1) = (middle_x - 1, middle_x + 2);
    for x in px0..=px1 {
        for y in floor..=floor + 4 {
            let frame = x == px0 || x == px1 || y == floor || y == floor + 4;
            grid.set(x, y, z0, Some(if frame { b.obsidian } else { b.nether_portal }));
        }
    }

    // Puerta de 2 de ancho al frente, alineada con el portal
    for x in px0 + 1..=px1 - 1 {
        for y in floor + 1..=floor + 3 {
            grid.set(x, y, z1, None);
        }
    }

    // Ventanas de 2 x 2 a los lados
    for z in z0 + 2..=z0 + 3 {
        for y in floor + 2..=floor + 3 {
            grid.set(x0, y, z, Some(b.glass));
            grid.set(x1, y, z, Some(b.glass));
        }
    }

    // Lamparas en las esquinas de adentro, junto a la puerta
    grid.set(x0 + 1, floor + 1, z1 - 1, Some(b.glowstone));
    grid.set(x1 - 1, floor + 1, z1 - 1, Some(b.glowstone));
}

/// Tragaluz: una columna de vidrio de 2 x 2 desde el pasto hasta el techo de la caverna,
/// por donde entra la luz del sol doblada por el vidrio
fn skylight(grid: &mut Grid, b: &Blocks) {
    for x in 3..=4 {
        for z in 4..=5 {
            let mut y = surface_height(x, z);
            while y > MIN_Y && !grid.is_air(x, y, z) {
                grid.set(x, y, z, Some(b.glass));
                y -= 1;
            }
        }
    }
}

/// Roble de Minecraft: tronco de 5 bloques y copa de hojas en capas
fn tree(grid: &mut Grid, b: &Blocks, x: i32, z: i32) {
    let base = surface_height(x, z) + 1;
    let top = base + 4;

    for (dy, radius) in [(-2, 2i32), (-1, 2), (0, 1), (1, 1)] {
        let y = top + dy;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                // Sin las esquinas, y algunas esquinas de las capas grandes al azar
                let corner = dx.abs() == radius && dz.abs() == radius;
                if corner && (radius == 1 || hash(x + dx, y, z + dz, 6) < 0.6) {
                    continue;
                }
                grid.set(x + dx, y, z + dz, Some(b.leaves));
            }
        }
    }
    for y in base..top {
        grid.set(x, y, z, Some(b.log));
    }
}

/// Alijo de un minero sobre la franja de obsidiana: bloques de oro y uno de diamante, que
/// reflejan la luz de la lava y la glowstone
fn treasure(grid: &mut Grid, b: &Blocks) {
    let spots = [(1, 6, b.gold), (2, 6, b.gold), (1, 7, b.gold), (1, 6, b.diamond)];
    for (x, z, block) in spots {
        // Primer lugar libre sobre el piso de esa columna
        let mut y = NIVEL_LIQUIDO + 1;
        while !grid.is_air(x, y, z) && y < -3 {
            y += 1;
        }
        grid.set(x, y, z, Some(block));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_hash_es_repetible_y_esta_entre_0_y_1() {
        for i in -20..20 {
            let h = hash(i, i * 3, -i, 9);
            assert_eq!(h, hash(i, i * 3, -i, 9));
            assert!((0.0..1.0).contains(&h));
        }
        assert_ne!(hash(1, 2, 3, 0), hash(1, 2, 3, 1));
    }

    #[test]
    fn el_ruido_es_suave_entre_puntos_cercanos() {
        let a = noise(3.30, -1.20, 7.70, 1);
        let b = noise(3.31, -1.20, 7.70, 1);
        assert!((a - b).abs() < 0.02);
        // En las esquinas enteras coincide con el hash
        assert!((noise(2.0, 5.0, -3.0, 1) - hash(2, 5, -3, 1)).abs() < 1e-6);
    }

    #[test]
    fn la_caverna_llega_al_frente_para_verse_por_el_corte() {
        let abierta = (MIN_X..=MAX_X).any(|x| (-8..=-3).any(|y| is_cave(x, y, MAX_Z)));
        assert!(abierta);
    }

    #[test]
    fn el_portal_de_la_cabana_se_ve_por_la_puerta() {
        let b = Blocks::load();
        let mut grid = Grid::new();
        terrain(&mut grid, &b);
        hut(&mut grid, &b);

        // El centro del portal es portal
        let p = portal_mina();
        let (px, py, pz) = (p.x.floor() as i32, p.y as i32, p.z as i32);
        assert_eq!(grid.get(px, py, pz), Some(b.nether_portal));
        assert_eq!(grid.get(px + 1, py, pz), Some(b.nether_portal));

        // Entre el portal y la puerta (incluida) todo es aire
        let (_, _, z0, z1) = CABANA;
        for z in z0 + 1..=z1 {
            assert!(grid.is_air(px, py, z), "z = {z}");
        }
    }

    #[test]
    fn la_colina_esta_atras_a_la_izquierda() {
        assert!(surface_height(-5, -6) >= 3);
        assert_eq!(surface_height(6, 6), SUPERFICIE);
    }
}
