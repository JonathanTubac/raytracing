//! Mapas normales: le dan relieve a una cara plana cambiando la normal punto por punto,
//! sin agregar geometria. La luz, el brillo y los reflejos usan esa normal, asi las juntas
//! de la piedra o las vetas de la madera se ven hundidas o salidas segun de donde llega la
//! luz.
//!
//! Minecraft no trae mapas normales, asi que se generan a partir de cada textura: el brillo
//! de cada pixel se toma como altura (lo claro sobresale, lo oscuro se hunde) y la
//! pendiente de esa altura inclina la normal.

use crate::math::{normalize, Vec3};
use crate::ray_intersect::{Face, Intersect};
use crate::texture::{ImageTexture, Texture};

/// Normales de una imagen en el espacio de la cara: `x` apunta hacia donde crece u, `y`
/// hacia donde crece v, y `z` hacia afuera de la superficie
#[derive(Debug)]
pub struct NormalImage {
    width: usize,
    height: usize,
    normals: Vec<Vec3>,
}

impl NormalImage {
    /// Mapa normal a partir del brillo de la imagen. `strength` es cuanto relieve tiene:
    /// 0 deja la cara plana y valores mas altos exageran las pendientes.
    pub fn from_height(image: &ImageTexture, strength: f32) -> &'static NormalImage {
        let (width, height) = (image.width(), image.height());
        let h = |x: usize, y: usize, dx: isize, dy: isize| {
            image.luminance(x as isize + dx, y as isize + dy)
        };

        let mut normals = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                // Filtro de Sobel: la pendiente en cada eje mira los 3 vecinos de cada lado,
                // con mas peso al del medio, asi un pixel suelto no genera un pico
                let du = (h(x, y, 1, -1) + 2.0 * h(x, y, 1, 0) + h(x, y, 1, 1))
                    - (h(x, y, -1, -1) + 2.0 * h(x, y, -1, 0) + h(x, y, -1, 1));
                let dv = (h(x, y, -1, 1) + 2.0 * h(x, y, 0, 1) + h(x, y, 1, 1))
                    - (h(x, y, -1, -1) + 2.0 * h(x, y, 0, -1) + h(x, y, 1, -1));

                // Si la altura sube hacia +u, la superficie mira hacia -u
                normals.push(normalize(&Vec3::new(-du * strength, -dv * strength, 1.0)));
            }
        }

        Box::leak(Box::new(NormalImage { width, height, normals }))
    }

    /// Normal del pixel mas cercano, igual que la textura, para que el relieve siga a los
    /// pixeles de Minecraft
    fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = (u.rem_euclid(1.0) * self.width as f32) as usize;
        let y = (v.clamp(0.0, 1.0) * self.height as f32) as usize;
        self.normals[y.min(self.height - 1) * self.width + x.min(self.width - 1)]
    }
}

/// Mapa normal de un material. Igual que su textura, puede ser uno para todas las caras o
/// uno distinto arriba, a los lados y abajo.
#[derive(Debug, Clone, Copy)]
pub enum NormalMap {
    Image(&'static NormalImage),
    Block {
        top: &'static NormalImage,
        side: &'static NormalImage,
        bottom: &'static NormalImage,
    },
}

impl NormalMap {
    /// Genera el mapa normal de cada imagen de la textura. Un color solido no tiene
    /// relieve, asi que devuelve `None`.
    pub fn from_texture(texture: &Texture, strength: f32) -> Option<NormalMap> {
        match texture {
            Texture::Image(image) => {
                Some(NormalMap::Image(NormalImage::from_height(image, strength)))
            }
            Texture::Block { top, side, bottom } => Some(NormalMap::Block {
                top: NormalImage::from_height(top, strength),
                side: NormalImage::from_height(side, strength),
                bottom: NormalImage::from_height(bottom, strength),
            }),
            _ => None,
        }
    }

    /// Normal del mapa en el espacio de la cara
    fn sample(&self, u: f32, v: f32, face: Face) -> Vec3 {
        match self {
            NormalMap::Image(image) => image.sample(u, v),
            NormalMap::Block { top, side, bottom } => match face {
                Face::Top => top.sample(u, v),
                Face::Side => side.sample(u, v),
                Face::Bottom => bottom.sample(u, v),
            },
        }
    }

    /// Normal en el mundo para ese impacto: la del mapa, llevada a la orientacion de la
    /// cara con su tangente (hacia donde crece u), bitangente (hacia donde crece v) y
    /// normal
    pub fn perturb(&self, hit: &Intersect) -> Vec3 {
        let n = self.sample(hit.u, hit.v, hit.face);
        normalize(&(hit.tangent * n.x + hit.bitangent * n.y + hit.normal * n.z))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::math::dot;

    // Imagen de gris con el brillo que da `altura` en cada pixel
    fn imagen(lado: usize, altura: impl Fn(usize, usize) -> u8) -> &'static ImageTexture {
        let mut pixels = Vec::new();
        for y in 0..lado {
            for x in 0..lado {
                let g = altura(x, y);
                pixels.push(Color::new(g, g, g));
            }
        }
        ImageTexture::from_pixels(lado, lado, pixels)
    }

    #[test]
    fn una_imagen_pareja_no_tiene_relieve() {
        let mapa = NormalImage::from_height(imagen(4, |_, _| 128), 3.0);
        for n in &mapa.normals {
            assert!((n - Vec3::new(0.0, 0.0, 1.0)).norm() < 1e-6);
        }
    }

    #[test]
    fn una_loma_inclina_la_normal_hacia_afuera_de_ella() {
        // Un pixel claro en el centro de una imagen oscura: a su derecha la altura baja
        // hacia +u, asi que ahi la superficie mira hacia +u; a su izquierda, hacia -u
        let loma = imagen(5, |x, y| if (x, y) == (2, 2) { 255 } else { 0 });
        let mapa = NormalImage::from_height(loma, 1.0);
        let derecha = mapa.normals[2 * 5 + 3];
        let izquierda = mapa.normals[2 * 5 + 1];
        let abajo = mapa.normals[3 * 5 + 2];

        assert!(derecha.x > 0.1 && izquierda.x < -0.1);
        assert!(abajo.y > 0.1);
        // Todas siguen mirando hacia afuera de la cara
        assert!(derecha.z > 0.0 && izquierda.z > 0.0);
    }

    #[test]
    fn mas_fuerza_da_mas_relieve() {
        let rampa = imagen(4, |x, _| (x * 60) as u8);
        let suave = NormalImage::from_height(rampa, 0.5).normals[5];
        let fuerte = NormalImage::from_height(rampa, 4.0).normals[5];
        assert!(fuerte.z < suave.z);
    }

    #[test]
    fn la_normal_del_mapa_se_orienta_con_la_cara() {
        // Cara que mira hacia +Z, con u creciendo hacia +X y v hacia -Y
        let rampa = imagen(3, |x, _| (x * 100) as u8);
        let mapa = NormalMap::Image(NormalImage::from_height(rampa, 2.0));
        let mut hit = Intersect::empty();
        hit.normal = Vec3::new(0.0, 0.0, 1.0);
        hit.tangent = Vec3::new(1.0, 0.0, 0.0);
        hit.bitangent = Vec3::new(0.0, -1.0, 0.0);
        hit.u = 0.5;
        hit.v = 0.5;

        let n = mapa.perturb(&hit);
        // La imagen se aclara hacia +u (+X), asi que la normal se inclina hacia -X
        assert!(n.x < -0.1, "{n:?}");
        assert!((n.norm() - 1.0).abs() < 1e-5 && dot(&n, &hit.normal) > 0.0);
    }
}
