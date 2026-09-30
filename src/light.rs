use crate::color::Color;
use crate::math::Vec3;

/// Point light with position, color and intensity
#[derive(Clone, Copy)]
pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    /// How far the light reaches. Beyond that it adds nothing.
    pub range: f32,
    /// Box of the blocks that emit the light (`None` for the sun)
    pub bounds: Option<(Vec3, Vec3)>,
}

// A block light reaches distance d as intensity / (1 + CAIDA * d^2)
const CAIDA: f32 = 0.5;

impl Light {
    /// Light that is equally strong at any distance, like the sun
    pub fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
            range: f32::INFINITY,
            bounds: None,
        }
    }

    /// Light from a single emissive block
    #[cfg(test)]
    pub fn from_block(center: Vec3, color: Color, intensity: f32, range: f32) -> Self {
        Light::from_blocks(&[center], color, intensity, range)
    }

    /// A single light for a group of neighboring emissive blocks, with their intensities added up
    pub fn from_blocks(centers: &[Vec3], color: Color, intensity: f32, range: f32) -> Self {
        let half = Vec3::new(0.5, 0.5, 0.5);
        let mut min = centers[0] - half;
        let mut max = centers[0] + half;
        let mut sum = Vec3::zeros();
        for center in centers {
            for axis in 0..3 {
                min[axis] = min[axis].min(center[axis] - 0.5);
                max[axis] = max[axis].max(center[axis] + 0.5);
            }
            sum += *center;
        }

        Light {
            position: sum * (1.0 / centers.len() as f32),
            color,
            intensity: intensity * centers.len() as f32,
            range,
            bounds: Some((min, max)),
        }
    }

    /// Fraction of the intensity that reaches that distance (1 for the sun, 0 beyond `range`)
    pub fn attenuation(&self, distance: f32) -> f32 {
        if !self.range.is_finite() {
            return 1.0;
        }
        let x = distance / self.range;
        if x >= 1.0 {
            return 0.0;
        }
        // Smooth window down to 0 at the range, so the light doesn't cut off abruptly
        let window = (1.0 - x * x * x * x).powi(2);
        window / (1.0 + CAIDA * distance * distance)
    }

    /// Whether the point is on (or inside) the blocks that emit the light
    pub fn contains(&self, point: &Vec3) -> bool {
        let Some((min, max)) = self.bounds else {
            return false;
        };
        (0..3).all(|axis| point[axis] >= min[axis] - 1e-3 && point[axis] <= max[axis] + 1e-3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blanca() -> Color {
        Color::rgb(1.0, 1.0, 1.0)
    }

    #[test]
    fn el_sol_no_se_debilita_con_la_distancia() {
        let sol = Light::new(Vec3::zeros(), blanca(), 1.0);
        assert_eq!(sol.attenuation(1.0), 1.0);
        assert_eq!(sol.attenuation(1000.0), 1.0);
    }

    #[test]
    fn la_luz_de_un_bloque_baja_con_la_distancia_hasta_apagarse() {
        let lampara = Light::from_block(Vec3::zeros(), blanca(), 1.0, 6.0);

        let mut anterior = lampara.attenuation(0.0);
        assert!((anterior - 1.0).abs() < 1e-6);
        for i in 1..=60 {
            let a = lampara.attenuation(i as f32 * 0.1);
            assert!(a < anterior, "a {} bloques", i as f32 * 0.1);
            anterior = a;
        }
        assert_eq!(lampara.attenuation(6.0), 0.0);
        assert_eq!(lampara.attenuation(20.0), 0.0);
    }

    #[test]
    fn el_bloque_de_la_luz_contiene_sus_caras_y_nada_mas() {
        let lampara = Light::from_block(Vec3::new(2.0, 0.0, 0.0), blanca(), 1.0, 6.0);

        assert!(lampara.contains(&Vec3::new(2.5, 0.2, -0.3)));
        assert!(lampara.contains(&Vec3::new(1.5, 0.5, 0.5)));
        assert!(!lampara.contains(&Vec3::new(3.1, 0.0, 0.0)));

        let sol = Light::new(Vec3::zeros(), blanca(), 1.0);
        assert!(!sol.contains(&Vec3::zeros()));
    }

    #[test]
    fn un_grupo_de_bloques_da_una_luz_en_su_centro_con_la_suma_de_intensidades() {
        let bloques = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
        ];
        let poza = Light::from_blocks(&bloques, blanca(), 0.5, 6.0);

        assert!((poza.position - Vec3::new(1.0, 0.0, 0.25)).norm() < 1e-6);
        assert!((poza.intensity - 2.0).abs() < 1e-6);
        // It touches any block of the group, not only the center one
        assert!(poza.contains(&Vec3::new(2.5, 0.0, 0.0)));
        assert!(poza.contains(&Vec3::new(-0.5, 0.3, 0.0)));
        assert!(!poza.contains(&Vec3::new(3.2, 0.0, 0.0)));
    }
}
