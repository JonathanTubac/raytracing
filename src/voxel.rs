//! Grilla de voxeles: el mundo como una matriz 3D de bloques de lado 1, igual que Minecraft.
//!
//! Probar un rayo contra cada bloque de la escena cuesta proporcional a la cantidad de
//! bloques. En una grilla basta con recorrer, en orden, solo las celdas que el rayo
//! atraviesa (algoritmo DDA de Amanatides y Woo): el primer bloque que aparece es el
//! impacto mas cercano. El costo pasa a depender de cuanto viaja el rayo, no de cuantos
//! bloques hay, asi que agregar bloques al diorama casi no lo hace mas lento.

use crate::cube::face_hit;
use crate::math::Vec3;
use crate::ray_intersect::{Intersect, Material, RayIntersect};

/// Celda vacia (aire). Las demas guardan 1 + el indice de su material en la paleta.
const AIRE: u8 = 0;

pub struct VoxelWorld {
    /// Esquina minima de la grilla en el mundo. La celda (i, j, k) ocupa el cubo que va
    /// de `min_corner + (i, j, k)` a `min_corner + (i + 1, j + 1, k + 1)`.
    min_corner: Vec3,
    size: [i32; 3],
    cells: Vec<u8>,
    /// Materiales distintos de la escena; cada celda guarda un indice a esta lista, asi un
    /// bloque ocupa un byte aunque su material sea grande
    palette: Vec<Material>,
}

impl VoxelWorld {
    /// Arma la grilla con los bloques dados: cada uno es la posicion entera de su centro y
    /// su material. Si dos bloques caen en la misma posicion queda el ultimo.
    pub fn new(blocks: &[([i32; 3], Material)]) -> VoxelWorld {
        // La grilla mide justo lo que ocupan los bloques
        let mut min = [i32::MAX; 3];
        let mut max = [i32::MIN; 3];
        for (position, _) in blocks {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        if blocks.is_empty() {
            (min, max) = ([0; 3], [-1; 3]);
        }
        let size = [max[0] - min[0] + 1, max[1] - min[1] + 1, max[2] - min[2] + 1];

        let mut world = VoxelWorld {
            // El bloque con centro en la posicion entera p ocupa de p - 0.5 a p + 0.5
            min_corner: Vec3::new(min[0] as f32, min[1] as f32, min[2] as f32)
                - Vec3::new(0.5, 0.5, 0.5),
            size,
            cells: vec![AIRE; (size[0] * size[1] * size[2]).max(0) as usize],
            palette: Vec::new(),
        };

        for (position, material) in blocks {
            let id = match world.palette.iter().position(|m| m == material) {
                Some(index) => index,
                None => {
                    world.palette.push(*material);
                    world.palette.len() - 1
                }
            };
            assert!(id < 255, "la paleta admite hasta 255 materiales distintos");

            let cell = [position[0] - min[0], position[1] - min[1], position[2] - min[2]];
            let index = world.index(cell).expect("el bloque esta dentro de la grilla");
            world.cells[index] = id as u8 + 1;
        }
        world
    }

    /// Cuantos bloques no vacios tiene la grilla
    pub fn block_count(&self) -> usize {
        self.cells.iter().filter(|&&c| c != AIRE).count()
    }

    fn index(&self, cell: [i32; 3]) -> Option<usize> {
        let [x, y, z] = cell;
        let [sx, sy, sz] = self.size;
        if x < 0 || y < 0 || z < 0 || x >= sx || y >= sy || z >= sz {
            return None;
        }
        Some(((y * sz + z) * sx + x) as usize)
    }

    /// Lo que hay en la celda; afuera de la grilla es todo aire
    fn get(&self, cell: [i32; 3]) -> u8 {
        self.index(cell).map_or(AIRE, |i| self.cells[i])
    }

    fn material(&self, block: u8) -> Material {
        self.palette[block as usize - 1]
    }

    fn is_transparent(&self, block: u8) -> bool {
        block != AIRE && self.material(block).transparency > 0.0
    }

    /// Impacto sobre la cara de `cell` perpendicular a `axis` (del lado maximo si
    /// `positive`), a distancia `t` sobre el rayo
    #[allow(clippy::too_many_arguments)]
    fn hit(
        &self,
        origin: &Vec3,
        direction: &Vec3,
        t: f32,
        cell: [i32; 3],
        axis: usize,
        positive: bool,
        block: u8,
    ) -> Intersect {
        let point = origin + direction * t;
        let corner = self.min_corner + Vec3::new(cell[0] as f32, cell[1] as f32, cell[2] as f32);
        let local = point - corner;
        let local = Vec3::new(
            local.x.clamp(0.0, 1.0),
            local.y.clamp(0.0, 1.0),
            local.z.clamp(0.0, 1.0),
        );
        face_hit(point, t, axis, positive, &local, self.material(block))
    }
}

impl RayIntersect for VoxelWorld {
    fn ray_intersect(&self, ray_origin: &Vec3, ray_direction: &Vec3) -> Intersect {
        // Todo se calcula relativo a la esquina de la grilla, donde la celda (i, j, k) va
        // de (i, j, k) a (i + 1, j + 1, k + 1)
        let o = ray_origin - self.min_corner;
        let d = ray_direction;
        let size = [self.size[0] as f32, self.size[1] as f32, self.size[2] as f32];

        // 1. Donde entra y sale el rayo de la caja que envuelve toda la grilla. Si no la
        //    toca, no hay nada que recorrer y se ve el cielo directo.
        let mut t_enter = 0.0f32;
        let mut t_exit = f32::INFINITY;
        let mut enter_axis = None;
        for axis in 0..3 {
            if d[axis].abs() < 1e-9 {
                if o[axis] < 0.0 || o[axis] > size[axis] {
                    return Intersect::empty();
                }
                continue;
            }
            let inv = 1.0 / d[axis];
            let (mut t0, mut t1) = (-o[axis] * inv, (size[axis] - o[axis]) * inv);
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            if t0 > t_enter {
                t_enter = t0;
                enter_axis = Some(axis);
            }
            t_exit = t_exit.min(t1);
        }
        if t_enter > t_exit {
            return Intersect::empty();
        }

        // 2. Celda de partida y cuanto hay que avanzar sobre el rayo para cruzar a la
        //    siguiente celda en cada eje (`t_max`) y para atravesar una celda entera
        //    (`t_delta`)
        let start = o + d * t_enter;
        let mut cell = [0i32; 3];
        let mut step = [0i32; 3];
        let mut t_max = [f32::INFINITY; 3];
        let mut t_delta = [f32::INFINITY; 3];
        for axis in 0..3 {
            cell[axis] = (start[axis].floor() as i32).clamp(0, self.size[axis] - 1);
            if d[axis] > 0.0 {
                step[axis] = 1;
                t_max[axis] = (cell[axis] as f32 + 1.0 - o[axis]) / d[axis];
                t_delta[axis] = 1.0 / d[axis];
            } else if d[axis] < 0.0 {
                step[axis] = -1;
                t_max[axis] = (cell[axis] as f32 - o[axis]) / d[axis];
                t_delta[axis] = -1.0 / d[axis];
            }
        }

        // Si el rayo nace dentro de un bloque (un rayo refractado dentro del agua, o uno
        // que sigue por el hueco de una hoja), ese bloque es el "medio" en el que viaja
        let inside = enter_axis.is_none();
        let medium = if inside { self.get(cell) } else { AIRE };
        let medium_is_transparent = self.is_transparent(medium);

        // Eje y distancia en que el rayo entro a la celda actual
        let mut axis = enter_axis.unwrap_or(0);
        let mut t = t_enter;
        let mut first = true;

        loop {
            // La celda de partida no se revisa si el rayo nace dentro de ella
            if !(first && inside) {
                let block = self.get(cell);
                // Cara por la que el rayo entro a esta celda: si avanza en positivo por el
                // eje, entro por el lado minimo de la celda
                let entered_positive = step[axis] < 0;
                let previous = {
                    let mut p = cell;
                    p[axis] -= step[axis];
                    p
                };

                if medium == AIRE {
                    // En el aire: el primer bloque que aparece es el impacto
                    if block != AIRE {
                        return self.hit(ray_origin, d, t, cell, axis, entered_positive, block);
                    }
                } else if medium_is_transparent {
                    // Dentro del agua o el vidrio: las caras entre dos bloques iguales no
                    // existen (un estanque es un solo volumen de agua, no muchos cubos).
                    // Al llegar a algo distinto, si es solido se ve eso; si no, el rayo sale
                    // del medio por su cara.
                    if block != medium {
                        if block != AIRE && !self.is_transparent(block) {
                            return self.hit(ray_origin, d, t, cell, axis, entered_positive, block);
                        }
                        return self.hit(ray_origin, d, t, previous, axis, !entered_positive, medium);
                    }
                } else {
                    // Dentro de un bloque opaco (por el hueco de una hoja): se ve la cara de
                    // ese bloque por donde sale el rayo
                    return self.hit(ray_origin, d, t, previous, axis, !entered_positive, medium);
                }
            }
            first = false;

            // 3. Paso a la celda vecina por el eje cuyo borde esta mas cerca
            axis = if t_max[0] < t_max[1] {
                if t_max[0] < t_max[2] { 0 } else { 2 }
            } else if t_max[1] < t_max[2] {
                1
            } else {
                2
            };
            t = t_max[axis];
            cell[axis] += step[axis];
            t_max[axis] += t_delta[axis];

            // Afuera de la grilla es aire. Si el rayo venia dentro de un bloque, todavia
            // hay que devolver la cara por la que sale; si no, ya no hay nada que ver.
            let outside = cell[axis] < 0 || cell[axis] >= self.size[axis];
            if outside && medium == AIRE {
                return Intersect::empty();
            }
            // Una direccion nula no avanza por ningun eje; sin esto no terminaria nunca
            if !t.is_finite() {
                return Intersect::empty();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::cube::Cube;
    use crate::math::normalize;
    use crate::texture::Texture;

    fn mate(r: u8) -> Material {
        Material::new(Texture::Solid(Color::new(r, 0, 0)))
    }

    fn agua() -> Material {
        Material {
            transparency: 0.8,
            refractive_index: 1.33,
            ..Material::new(Texture::Solid(Color::new(0, 0, 200)))
        }
    }

    /// Generador de numeros pseudoaleatorios simple (LCG), para no depender de librerias
    struct Azar(u64);

    impl Azar {
        fn siguiente(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (self.0 >> 40) as f32 / (1u64 << 24) as f32
        }

        fn entre(&mut self, a: f32, b: f32) -> f32 {
            a + (b - a) * self.siguiente()
        }
    }

    #[test]
    fn da_el_mismo_impacto_que_probar_cubo_por_cubo() {
        // Mundo al azar de bloques opacos de varios colores, y cientos de rayos al azar
        // desde afuera: la grilla tiene que dar el mismo impacto que la fuerza bruta
        let mut azar = Azar(7);
        let mut bloques = Vec::new();
        for x in -4..4 {
            for y in -3..3 {
                for z in -4..4 {
                    if azar.siguiente() < 0.15 {
                        bloques.push(([x, y, z], mate(1 + (azar.siguiente() * 5.0) as u8)));
                    }
                }
            }
        }
        let mundo = VoxelWorld::new(&bloques);
        let cubos: Vec<Cube> = bloques
            .iter()
            .map(|(p, m)| Cube::new(Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32), 1.0, *m))
            .collect();

        let mut con_impacto = 0;
        for _ in 0..500 {
            let origen = Vec3::new(azar.entre(-12.0, 12.0), azar.entre(-8.0, 8.0), 12.0);
            let objetivo = Vec3::new(azar.entre(-4.0, 4.0), azar.entre(-3.0, 3.0), azar.entre(-4.0, 4.0));
            let direccion = normalize(&(objetivo - origen));

            let grilla = mundo.ray_intersect(&origen, &direccion);
            let fuerza_bruta = cubos
                .iter()
                .map(|c| c.ray_intersect(&origen, &direccion))
                .filter(|h| h.is_intersecting)
                .min_by(|a, b| a.distance.total_cmp(&b.distance));

            match fuerza_bruta {
                None => assert!(!grilla.is_intersecting),
                Some(esperado) => {
                    con_impacto += 1;
                    assert!(grilla.is_intersecting);
                    assert!((grilla.distance - esperado.distance).abs() < 1e-3);
                    assert!((grilla.normal - esperado.normal).norm() < 1e-5);
                    assert!((grilla.u - esperado.u).abs() < 1e-3 && (grilla.v - esperado.v).abs() < 1e-3);
                    assert_eq!(grilla.material, esperado.material);
                }
            }
        }
        // Que la prueba no pase solo porque los rayos no chocan con nada
        assert!(con_impacto > 100, "{con_impacto}");
    }

    #[test]
    fn un_rayo_que_no_toca_la_grilla_no_choca() {
        let mundo = VoxelWorld::new(&[([0, 0, 0], mate(1))]);
        let hit = mundo.ray_intersect(&Vec3::new(5.0, 5.0, 5.0), &Vec3::new(0.0, 1.0, 0.0));
        assert!(!hit.is_intersecting);
    }

    #[test]
    fn un_bloque_detras_de_otro_queda_tapado() {
        let mundo = VoxelWorld::new(&[([0, 0, -3], mate(1)), ([0, 0, -6], mate(2))]);
        let hit = mundo.ray_intersect(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0));

        assert!((hit.distance - 2.5).abs() < 1e-5);
        assert_eq!(hit.material, mate(1));
    }

    #[test]
    fn el_agua_de_varios_bloques_es_un_solo_volumen() {
        // Tres bloques de agua en fila y piedra al final
        let bloques = [
            ([0, 0, -2], agua()),
            ([0, 0, -3], agua()),
            ([0, 0, -4], agua()),
            ([0, 0, -5], mate(9)),
        ];
        let mundo = VoxelWorld::new(&bloques);

        // Desde afuera se ve la superficie del agua
        let entrada = mundo.ray_intersect(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0));
        assert!((entrada.distance - 1.5).abs() < 1e-5);
        assert_eq!(entrada.material, agua());

        // Desde adentro, el rayo cruza las caras entre bloques de agua y llega a la piedra
        let dentro = Vec3::new(0.0, 0.0, -1.6);
        let fondo = mundo.ray_intersect(&dentro, &Vec3::new(0.0, 0.0, -1.0));
        assert!((fondo.distance - 2.9).abs() < 1e-4);
        assert_eq!(fondo.material, mate(9));

        // Y hacia arriba sale por la superficie, con la normal hacia afuera del agua
        let salida = mundo.ray_intersect(&dentro, &Vec3::new(0.0, 1.0, 0.0));
        assert!((salida.distance - 0.5).abs() < 1e-4);
        assert_eq!(salida.material, agua());
        assert!((salida.normal - Vec3::new(0.0, 1.0, 0.0)).norm() < 1e-5);
    }

    #[test]
    fn desde_adentro_de_un_bloque_opaco_se_ve_la_cara_de_salida() {
        let mundo = VoxelWorld::new(&[([0, 0, 0], mate(1)), ([0, 0, -1], mate(2))]);
        let hit = mundo.ray_intersect(&Vec3::new(0.0, 0.0, 0.2), &Vec3::new(0.0, 0.0, -1.0));

        // Sale del bloque en el que esta, aunque al lado haya otro
        assert!((hit.distance - 0.7).abs() < 1e-5);
        assert_eq!(hit.material, mate(1));
        assert!((hit.normal - Vec3::new(0.0, 0.0, -1.0)).norm() < 1e-5);
    }

    #[test]
    fn la_paleta_guarda_cada_material_una_sola_vez() {
        let bloques: Vec<_> = (0..50).map(|i| ([i, 0, 0], mate((i % 3) as u8))).collect();
        let mundo = VoxelWorld::new(&bloques);

        assert_eq!(mundo.palette.len(), 3);
        assert_eq!(mundo.block_count(), 50);
    }
}
