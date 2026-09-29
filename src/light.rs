use crate::color::Color;
use crate::math::Vec3;

/// Luz puntual: ilumina desde `position` con el color `color` y una fuerza `intensity`
/// (1.0 = normal).
///
/// Hay dos tipos: el sol, que no se debilita con la distancia (`range` infinito), y las
/// luces de los bloques emisivos, que viven dentro de un bloque y solo alumbran hasta
/// `range` bloques de distancia.
pub struct Light {
    pub position: Vec3,
    pub color: Color,
    pub intensity: f32,
    /// Hasta donde llega la luz. Mas alla no aporta nada.
    pub range: f32,
    /// Mitad del lado del bloque que emite la luz (0 si no sale de un bloque)
    pub radius: f32,
}

// Que tan rapido se debilita la luz de un bloque: a distancia d llega
// intensity / (1 + CAIDA * d^2), como una luz real que se reparte en una esfera cada vez
// mas grande
const CAIDA: f32 = 0.5;

impl Light {
    /// Luz que alumbra igual a cualquier distancia, como el sol
    pub fn new(position: Vec3, color: Color, intensity: f32) -> Self {
        Light {
            position,
            color,
            intensity,
            range: f32::INFINITY,
            radius: 0.0,
        }
    }

    /// Luz que sale de un bloque emisivo de lado 1 con centro en `center`, y alumbra hasta
    /// `range` bloques de distancia
    pub fn from_block(center: Vec3, color: Color, intensity: f32, range: f32) -> Self {
        Light {
            position: center,
            color,
            intensity,
            range,
            radius: 0.5,
        }
    }

    /// Que parte de la intensidad llega a esa distancia: 1 para el sol y menos de 1 para
    /// las luces de bloque, hasta llegar a 0 justo en `range`
    pub fn attenuation(&self, distance: f32) -> f32 {
        if !self.range.is_finite() {
            return 1.0;
        }
        let x = distance / self.range;
        if x >= 1.0 {
            return 0.0;
        }
        // La ventana baja suave hasta 0 en el alcance; sin ella la luz se cortaria de golpe
        // y se veria un circulo marcado alrededor de cada bloque
        let window = (1.0 - x * x * x * x).powi(2);
        window / (1.0 + CAIDA * distance * distance)
    }

    /// Si el punto esta sobre (o dentro de) el bloque que emite la luz. Un rayo de sombra
    /// que llega ahi ya llego a la luz: el bloque no se tapa a si mismo.
    pub fn contains(&self, point: &Vec3) -> bool {
        let d = point - self.position;
        let limit = self.radius + 1e-3;
        self.radius > 0.0 && d.x.abs() <= limit && d.y.abs() <= limit && d.z.abs() <= limit
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
}
