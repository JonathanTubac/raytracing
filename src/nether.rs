//! The Nether: the island on the other side of the portal

use crate::blocks::Blocks;
use crate::diorama::{hash, noise, Grid, MAX_X, MAX_Y, MAX_Z, MIN_X, MIN_Y, MIN_Z, SUPERFICIE};
use crate::math::Vec3;
use crate::ray_intersect::Material;

// Surface level of the lava lake
const NIVEL_LAVA: i32 = -4;

// Arrival portal frame: x columns (inclusive) and its z row
const PORTAL_X: (i32, i32) = (-9, -6);
const PORTAL_Z: i32 = 3;

/// Center of the arrival portal; the camera appears here when coming from the mine
pub fn portal_nether() -> Vec3 {
    let (x0, x1) = PORTAL_X;
    Vec3::new((x0 + x1) as f32 / 2.0, SUPERFICIE as f32 + 2.0, PORTAL_Z as f32)
}

/// Surface height: wavy with noise, and flat around the portal
fn surface_height(x: i32, z: i32) -> i32 {
    if is_platform(x, z) {
        return SUPERFICIE;
    }
    let bumps = (noise(x as f32 * 0.25, 0.0, z as f32 * 0.25, 20) - 0.5) * 3.0;
    SUPERFICIE + bumps.round() as i32
}

/// Flat platform where the arrival portal stands
fn is_platform(x: i32, z: i32) -> bool {
    (MIN_X..=-4).contains(&x) && (1..=MAX_Z).contains(&z)
}

/// Lava lake: an oval at the front right
fn is_lake(x: i32, z: i32) -> bool {
    let (dx, dz) = ((x as f32 - 4.0) / 6.5, (z as f32 - 6.0) / 4.5);
    let wobble = (noise(x as f32 * 0.4, 5.0, z as f32 * 0.4, 21) - 0.5) * 0.5;
    dx * dx + dz * dz + wobble < 1.0
}

/// Builds the whole Nether: the list of blocks with their position and material
pub fn nether(b: &Blocks) -> Vec<([i32; 3], Material)> {
    let mut grid = Grid::new();

    body(&mut grid, b);
    lava_lake(&mut grid, b);
    soul_valley(&mut grid, b);
    crimson_forest(&mut grid, b);
    bridge(&mut grid, b);
    floating_glowstone(&mut grid, b);
    arrival_portal(&mut grid, b);

    grid.into_blocks()
}

/// Netherrack with ores, a blackstone base and stalactites
fn body(grid: &mut Grid, b: &Blocks) {
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            let top = surface_height(x, z);
            let bottom = MIN_Y + 2 + (noise(x as f32 * 0.3, 0.0, z as f32 * 0.3, 22) * 3.0) as i32;

            for y in bottom..=top {
                let r = hash(x, y, z, 23);
                let block = if y <= bottom + 1 {
                    if r < 0.3 { b.basalt } else { b.blackstone }
                } else if r < 0.07 {
                    b.quartz_ore
                } else if r > 0.95 {
                    b.nether_gold_ore
                } else if y < -8 && (0.5..0.52).contains(&r) {
                    b.ancient_debris
                } else {
                    b.netherrack
                };
                grid.set(x, y, z, Some(block));
            }

            // Netherrack stalactites under the island
            if hash(x, 0, z, 24) < 0.18 {
                let length = 1 + (hash(x, 1, z, 24) * 4.0) as i32;
                for y in (bottom - length).max(MIN_Y)..bottom {
                    grid.set(x, y, z, Some(b.netherrack));
                }
            }
        }
    }
}

/// Two block deep lava lake, with magma on the shore and at the bottom
fn lava_lake(grid: &mut Grid, b: &Blocks) {
    for x in MIN_X..=MAX_X {
        for z in MIN_Z..=MAX_Z {
            if !is_lake(x, z) {
                // Shore: magma on the surface next to the lake
                let shore = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .any(|(dx, dz)| is_lake(x + dx, z + dz));
                if shore && hash(x, 0, z, 25) < 0.6 {
                    grid.set(x, surface_height(x, z), z, Some(b.magma));
                }
                continue;
            }

            for y in NIVEL_LAVA + 1..=MAX_Y {
                grid.set(x, y, z, None);
            }
            grid.set(x, NIVEL_LAVA, z, Some(b.lava));
            grid.set(x, NIVEL_LAVA - 1, z, Some(b.lava));
            if hash(x, 1, z, 26) < 0.35 {
                grid.set(x, NIVEL_LAVA - 2, z, Some(b.magma));
            }
        }
    }
}

/// Soul sand valley at the back left, with basalt pillars of different heights
fn soul_valley(grid: &mut Grid, b: &Blocks) {
    for x in MIN_X..=-3 {
        for z in MIN_Z..=-2 {
            let top = surface_height(x, z);
            grid.set(x, top, z, Some(b.soul_sand));
            grid.set(x, top - 1, z, Some(b.soul_sand));

            if hash(x, 2, z, 27) < 0.13 {
                let height = 2 + (hash(x, 3, z, 27) * 4.0) as i32;
                for y in top + 1..=top + height {
                    grid.set(x, y, z, Some(b.basalt));
                }
            }
        }
    }
}

/// Crimson forest at the back right: nylium on the ground and two huge fungi
fn crimson_forest(grid: &mut Grid, b: &Blocks) {
    for x in 0..=MAX_X {
        for z in MIN_Z..=0 {
            if !is_lake(x, z) {
                grid.set(x, surface_height(x, z), z, Some(b.crimson_nylium));
            }
        }
    }
    huge_fungus(grid, b, 4, -6, 5);
    huge_fungus(grid, b, 8, -1, 4);
}

/// Huge crimson fungus with shroomlights under the cap
fn huge_fungus(grid: &mut Grid, b: &Blocks, x: i32, z: i32, height: i32) {
    let base = surface_height(x, z) + 1;
    let top = base + height;

    for (dy, radius) in [(2, 1i32), (1, 2), (0, 3), (-1, 3)] {
        let y = top + dy;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let distance = dx.abs().max(dz.abs());
                let corner = dx.abs() == radius && dz.abs() == radius;
                // The two bottom layers are just the hanging rim, hollow inside
                let skirt_only = dy <= 0 && distance < radius;
                if corner || skirt_only {
                    continue;
                }
                // The lowest rim is uneven: some blocks are missing
                if dy == -1 && hash(x + dx, y, z + dz, 28) < 0.35 {
                    continue;
                }
                grid.set(x + dx, y, z + dz, Some(b.nether_wart));
            }
        }
    }

    // Shroomlights hanging under the cap, around the stem
    for dx in -2..=2 {
        for dz in -2..=2 {
            if (dx != 0 || dz != 0) && hash(x + dx, top, z + dz, 29) < 0.3 {
                grid.set(x + dx, top, z + dz, Some(b.shroomlight));
            }
        }
    }

    for y in base..=top + 1 {
        grid.set(x, y, z, Some(b.crimson_stem));
    }
}

/// Fortress bridge over the lake, with pillars and an arch
fn bridge(grid: &mut Grid, b: &Blocks) {
    let deck = SUPERFICIE;
    let (z0, z1) = (4, 6);
    for x in -3..=MAX_X {
        for z in z0..=z1 {
            grid.set(x, deck, z, Some(b.nether_bricks));
            for y in deck + 1..=deck + 3 {
                grid.set(x, y, z, None);
            }
        }
        grid.set(x, deck + 1, z0, Some(b.nether_bricks));
        grid.set(x, deck + 1, z1, Some(b.nether_bricks));
    }

    // Pillars down to the bottom of the lake
    for x in [1, 6] {
        for z in z0..=z1 {
            for y in NIVEL_LAVA - 2..deck {
                grid.set(x, y, z, Some(b.nether_bricks));
            }
        }
    }

    // Arch at the end of the bridge
    for y in deck + 1..=deck + 4 {
        grid.set(MAX_X, y, z0, Some(b.nether_bricks));
        grid.set(MAX_X, y, z1, Some(b.nether_bricks));
    }
    for z in z0..=z1 {
        grid.set(MAX_X, deck + 4, z, Some(b.nether_bricks));
    }
    // Bridge entrance left open, without a railing
    grid.set(-3, deck + 1, z0 + 1, None);
}

/// Floating rock with glowstone underneath
fn floating_glowstone(grid: &mut Grid, b: &Blocks) {
    let (cx, cy, cz) = (-4.0, 8.5, -3.0);
    for x in -9..=1 {
        for y in 6..=MAX_Y {
            for z in -8..=2 {
                let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                let (dx, dy, dz) = ((fx - cx) / 3.5, (fy - cy) / 1.4, (fz - cz) / 2.8);
                let wobble = (noise(fx * 0.5, fy * 0.5, fz * 0.5, 30) - 0.5) * 0.6;
                if dx * dx + dy * dy + dz * dz + wobble < 1.0 {
                    grid.set(x, y, z, Some(b.netherrack));
                }
            }
        }
    }

    for x in -9..=1 {
        for z in -8..=2 {
            for y in 6..MAX_Y {
                let underside = grid.is_air(x, y, z) && !grid.is_air(x, y + 1, z);
                if underside && grid.get(x, y + 1, z) == Some(b.netherrack) {
                    if hash(x, y, z, 31) < 0.55 {
                        grid.set(x, y + 1, z, Some(b.glowstone));
                    }
                    if hash(x, y, z, 32) < 0.2 {
                        grid.set(x, y, z, Some(b.glowstone));
                    }
                }
            }
        }
    }
}

/// Arrival portal with crying obsidian around it
fn arrival_portal(grid: &mut Grid, b: &Blocks) {
    let (x0, x1) = PORTAL_X;
    let floor = SUPERFICIE;

    // Platform and air above it
    for x in x0 - 1..=x1 + 2 {
        for z in PORTAL_Z - 1..=PORTAL_Z + 3 {
            grid.set(x, floor, z, Some(b.blackstone));
            for y in floor + 1..=floor + 5 {
                grid.set(x, y, z, None);
            }
        }
    }

    // 4 x 5 obsidian frame, with the 2 x 3 portal inside
    for x in x0..=x1 {
        for y in floor..=floor + 4 {
            let frame = x == x0 || x == x1 || y == floor || y == floor + 4;
            let block = if !frame {
                b.nether_portal
            } else if hash(x, y, PORTAL_Z, 33) < 0.3 {
                b.crying_obsidian
            } else {
                b.obsidian
            };
            grid.set(x, y, PORTAL_Z, Some(block));
        }
    }

    for (x, z) in [(x0 - 1, PORTAL_Z + 1), (x1 + 1, PORTAL_Z + 2), (x1 + 2, PORTAL_Z)] {
        grid.set(x, floor + 1, z, Some(b.crying_obsidian));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_portal_de_llegada_tiene_portal_en_su_centro_y_espacio_delante() {
        let b = Blocks::load();
        let mut grid = Grid::new();
        body(&mut grid, &b);
        arrival_portal(&mut grid, &b);

        let p = portal_nether();
        let (px, py, pz) = (p.x.floor() as i32, p.y as i32, p.z as i32);
        assert_eq!(grid.get(px, py, pz), Some(b.nether_portal));
        assert_eq!(grid.get(px + 1, py, pz), Some(b.nether_portal));
        for z in pz + 1..=pz + 3 {
            assert!(grid.is_air(px, py, z), "z = {z}");
        }
    }

    #[test]
    fn el_lago_llega_al_frente_para_verse_por_el_corte() {
        assert!((0..=MAX_X).any(|x| is_lake(x, MAX_Z)));
        assert!(!is_lake(-9, -9));
    }
}
