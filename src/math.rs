use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Sub};

/// 3 component vector: positions, directions and normals
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub const fn zeros() -> Self {
        Vec3::new(0.0, 0.0, 0.0)
    }

    /// Vector length
    pub fn norm(&self) -> f32 {
        dot(self, self).sqrt()
    }

    /// Component-wise division
    pub fn component_div(&self, other: &Vec3) -> Vec3 {
        Vec3::new(self.x / other.x, self.y / other.y, self.z / other.z)
    }
}

pub fn dot(a: &Vec3, b: &Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Vector perpendicular to `a` and `b` (right-hand rule)
pub fn cross(a: &Vec3, b: &Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// Same vector with length 1. The zero vector is left as is to avoid NaN.
pub fn normalize(v: &Vec3) -> Vec3 {
    let norm = v.norm();
    if norm > 0.0 { *v * (1.0 / norm) } else { *v }
}

impl Index<usize> for Vec3 {
    type Output = f32;

    fn index(&self, axis: usize) -> &f32 {
        match axis {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vec3 solo tiene los ejes 0, 1 y 2, no {axis}"),
        }
    }
}

impl IndexMut<usize> for Vec3 {
    fn index_mut(&mut self, axis: usize) -> &mut f32 {
        match axis {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("Vec3 solo tiene los ejes 0, 1 y 2, no {axis}"),
        }
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, other: Vec3) {
        *self = *self + other;
    }
}

// Operators between `Vec3` and `&Vec3` in any combination
macro_rules! operaciones_entre_vectores {
    ($($izq:ty, $der:ty);*) => {$(
        impl Add<$der> for $izq {
            type Output = Vec3;
            fn add(self, o: $der) -> Vec3 {
                Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
            }
        }

        impl Sub<$der> for $izq {
            type Output = Vec3;
            fn sub(self, o: $der) -> Vec3 {
                Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
            }
        }
    )*};
}
operaciones_entre_vectores!(Vec3, Vec3; Vec3, &Vec3; &Vec3, Vec3; &Vec3, &Vec3);

macro_rules! operaciones_con_escalar {
    ($($v:ty),*) => {$(
        impl Mul<f32> for $v {
            type Output = Vec3;
            fn mul(self, s: f32) -> Vec3 {
                Vec3::new(self.x * s, self.y * s, self.z * s)
            }
        }

        impl Mul<$v> for f32 {
            type Output = Vec3;
            fn mul(self, v: $v) -> Vec3 {
                v * self
            }
        }

        impl Neg for $v {
            type Output = Vec3;
            fn neg(self) -> Vec3 {
                Vec3::new(-self.x, -self.y, -self.z)
            }
        }
    )*};
}
operaciones_con_escalar!(Vec3, &Vec3);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suma_resta_y_escala() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(&b - &a, Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(2.0 * &a, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn producto_punto_y_cruz() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);

        assert_eq!(dot(&x, &y), 0.0);
        assert_eq!(dot(&x, &x), 1.0);
        assert_eq!(cross(&x, &y), Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn normalizar_deja_largo_uno_y_respeta_el_cero() {
        let v = normalize(&Vec3::new(3.0, 0.0, 4.0));
        assert!((v.norm() - 1.0).abs() < 1e-6);
        assert_eq!(normalize(&Vec3::zeros()), Vec3::zeros());
    }

    #[test]
    fn se_puede_leer_y_escribir_por_eje() {
        let mut v = Vec3::new(1.0, 2.0, 3.0);
        v[1] = 7.0;
        assert_eq!((v[0], v[1], v[2]), (1.0, 7.0, 3.0));
    }
}
