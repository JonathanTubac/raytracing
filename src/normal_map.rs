//! Normal maps generated from the brightness of each texture

use crate::math::{normalize, Vec3};
use crate::ray_intersect::{Face, Intersect};
use crate::texture::{ImageTexture, Texture};

/// Normals in face space: x along u, y along v, z pointing out
#[derive(Debug, PartialEq)]
pub struct NormalImage {
    width: usize,
    height: usize,
    normals: Vec<Vec3>,
    /// Blend neighboring pixels when sampling (water) or use the nearest one (stone)
    smooth: bool,
}

impl NormalImage {
    /// Normal map from the image brightness
    pub fn from_height(image: &ImageTexture, strength: f32) -> &'static NormalImage {
        NormalImage::build(image, strength, 0)
    }

    /// Like `from_height`, but blurred and smooth, for water
    pub fn from_height_smooth(
        image: &ImageTexture,
        strength: f32,
        blur: usize,
    ) -> &'static NormalImage {
        NormalImage::build(image, strength, blur)
    }

    fn build(image: &ImageTexture, strength: f32, blur: usize) -> &'static NormalImage {
        let (width, height) = (image.width(), image.height());

        // Height of each pixel (its brightness), blurred if needed
        let mut heights: Vec<f32> = (0..width * height)
            .map(|i| image.luminance((i % width) as isize, (i / width) as isize))
            .collect();
        if blur > 0 {
            // Two averaging passes give a rounder curve than one
            for _ in 0..2 {
                heights = box_blur(&heights, width, height, blur);
            }
        }
        let h = |x: usize, y: usize, dx: isize, dy: isize| {
            let x = (x as isize + dx).rem_euclid(width as isize) as usize;
            let y = (y as isize + dy).rem_euclid(height as isize) as usize;
            heights[y * width + x]
        };

        let mut normals = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                // Sobel filter: slope on each axis from the neighbors on each side
                let du = (h(x, y, 1, -1) + 2.0 * h(x, y, 1, 0) + h(x, y, 1, 1))
                    - (h(x, y, -1, -1) + 2.0 * h(x, y, -1, 0) + h(x, y, -1, 1));
                let dv = (h(x, y, -1, 1) + 2.0 * h(x, y, 0, 1) + h(x, y, 1, 1))
                    - (h(x, y, -1, -1) + 2.0 * h(x, y, 0, -1) + h(x, y, 1, -1));

                // If the height rises toward +u, the surface faces -u
                normals.push(normalize(&Vec3::new(-du * strength, -dv * strength, 1.0)));
            }
        }

        Box::leak(Box::new(NormalImage {
            width,
            height,
            normals,
            smooth: blur > 0,
        }))
    }

    /// Normal at point (u, v)
    fn sample(&self, u: f32, v: f32) -> Vec3 {
        let (w, h) = (self.width, self.height);
        if !self.smooth {
            let x = (u.rem_euclid(1.0) * w as f32) as usize;
            let y = (v.clamp(0.0, 1.0) * h as f32) as usize;
            return self.normals[y.min(h - 1) * w + x.min(w - 1)];
        }

        let x = u.rem_euclid(1.0) * w as f32 - 0.5;
        let y = v.rem_euclid(1.0) * h as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (tx, ty) = (x - x0, y - y0);
        let at = |px: f32, py: f32| {
            let px = (px as isize).rem_euclid(w as isize) as usize;
            let py = (py as isize).rem_euclid(h as isize) as usize;
            self.normals[py * w + px]
        };
        let top = at(x0, y0) * (1.0 - tx) + at(x0 + 1.0, y0) * tx;
        let bottom = at(x0, y0 + 1.0) * (1.0 - tx) + at(x0 + 1.0, y0 + 1.0) * tx;
        normalize(&(top * (1.0 - ty) + bottom * ty))
    }
}

/// Average of each pixel with its neighbors, wrapping around the edges
fn box_blur(values: &[f32], width: usize, height: usize, radius: usize) -> Vec<f32> {
    let r = radius as isize;
    let count = ((2 * r + 1) * (2 * r + 1)) as f32;
    (0..width * height)
        .map(|i| {
            let (x, y) = ((i % width) as isize, (i / width) as isize);
            let mut sum = 0.0;
            for dy in -r..=r {
                for dx in -r..=r {
                    let px = (x + dx).rem_euclid(width as isize) as usize;
                    let py = (y + dy).rem_euclid(height as isize) as usize;
                    sum += values[py * width + px];
                }
            }
            sum / count
        })
        .collect()
}

/// Normal map of a material
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NormalMap {
    Image(&'static NormalImage),
    Block {
        top: &'static NormalImage,
        side: &'static NormalImage,
        bottom: &'static NormalImage,
    },
}

impl NormalMap {
    /// Generates the normal map of each image of the texture
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

    /// Soft wave normal map, for water
    pub fn smooth_from_texture(
        texture: &Texture,
        strength: f32,
        blur: usize,
    ) -> Option<NormalMap> {
        match texture {
            Texture::Image(image) => {
                let smooth = NormalImage::from_height_smooth(image, strength, blur);
                Some(NormalMap::Image(smooth))
            }
            _ => None,
        }
    }

    /// Normal from the map in face space
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

    /// World normal: the map normal, oriented with the tangent, bitangent and normal
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

    // Gray image with the brightness given by `altura` at each pixel
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
        // Around a bright pixel, the surface faces away from it
        let loma = imagen(5, |x, y| if (x, y) == (2, 2) { 255 } else { 0 });
        let mapa = NormalImage::from_height(loma, 1.0);
        let derecha = mapa.normals[2 * 5 + 3];
        let izquierda = mapa.normals[2 * 5 + 1];
        let abajo = mapa.normals[3 * 5 + 2];

        assert!(derecha.x > 0.1 && izquierda.x < -0.1);
        assert!(abajo.y > 0.1);
        // They all still face out of the face
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
    fn el_mapa_suave_cambia_de_a_poco_entre_pixeles_vecinos() {
        // Irregular per-pixel noise, like the water texture
        let ruido = imagen(8, |x, y| ((x * 7 + y * 13 + x * y) % 5 * 60) as u8);
        let comun = NormalImage::from_height(ruido, 1.0);
        let suave = NormalImage::from_height_smooth(ruido, 1.0, 2);

        // How much the normal changes when moving slightly across the face
        let salto = |mapa: &NormalImage| {
            (0..64)
                .map(|i| {
                    let u = i as f32 / 64.0;
                    (mapa.sample(u, 0.3) - mapa.sample(u + 1.0 / 64.0, 0.3)).norm()
                })
                .fold(0.0, f32::max)
        };
        assert!(salto(suave) < salto(comun) * 0.5 + 1e-6);
        // And the smooth one has no sudden jumps
        assert!(salto(suave) < 0.2, "{}", salto(suave));
    }

    #[test]
    fn la_normal_del_mapa_se_orienta_con_la_cara() {
        // Face looking toward +Z, with u growing toward +X and v toward -Y
        let rampa = imagen(3, |x, _| (x * 100) as u8);
        let mapa = NormalMap::Image(NormalImage::from_height(rampa, 2.0));
        let mut hit = Intersect::empty();
        hit.normal = Vec3::new(0.0, 0.0, 1.0);
        hit.tangent = Vec3::new(1.0, 0.0, 0.0);
        hit.bitangent = Vec3::new(0.0, -1.0, 0.0);
        hit.u = 0.5;
        hit.v = 0.5;

        let n = mapa.perturb(&hit);
        // The image gets brighter toward +u (+X), so the normal tilts toward -X
        assert!(n.x < -0.1, "{n:?}");
        assert!((n.norm() - 1.0).abs() < 1e-5 && dot(&n, &hit.normal) > 0.0);
    }
}
