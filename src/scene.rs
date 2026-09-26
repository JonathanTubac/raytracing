use nalgebra_glm::Vec3;

use crate::render::FOCAL;
use crate::sphere::Sphere;

// Colores del gopher (tomados de la imagen de referencia)
const AZUL: u32 = 0x74CEDD;
const PIEL: u32 = 0xF7D3A2;
const NEGRO: u32 = 0x050708;
const BLANCO: u32 = 0xFFFFFF;

// La referencia es de 600x600 pixeles y la escena se renderiza a ese mismo tamano
pub const IMG: f32 = 600.0;

/// Crea una esfera que, vista desde la camara, aparece centrada en el pixel (px, py)
/// de la imagen de referencia con un radio de `r` pixeles. `depth` es que tan lejos
/// esta de la camara: las esferas con menor `depth` tapan a las de mayor `depth`.
fn esfera(px: f32, py: f32, r: f32, depth: f32, color: u32) -> Sphere {
    let screen_x = 2.0 * px / IMG - 1.0;
    let screen_y = 1.0 - 2.0 * py / IMG;
    let center = Vec3::new(screen_x * depth, screen_y * depth, -FOCAL * depth);

    // El plano de imagen esta a distancia FOCAL y mide 2 de alto, asi que r pixeles
    // equivalen a tan(beta) = r / (IMG / 2) / FOCAL. Con eso sacamos el radio real de la esfera.
    let k = r / (IMG / 2.0) / FOCAL;
    let radius = center.norm() * k / (1.0 + k * k).sqrt();

    Sphere {
        center,
        radius,
        color,
    }
}

pub fn gopher() -> Vec<Sphere> {
    vec![
        // Manos y pies (al fondo, detras del cuerpo)
        esfera(145.0, 312.0, 15.0, 16.0, PIEL),
        esfera(456.0, 311.0, 15.0, 16.0, PIEL),
        esfera(188.0, 516.0, 19.0, 16.0, PIEL),
        esfera(410.0, 513.0, 18.0, 16.0, PIEL),
        // Orejas y su interior
        esfera(166.0, 121.0, 27.0, 15.0, AZUL),
        esfera(425.0, 115.0, 27.0, 15.0, AZUL),
        esfera(165.0, 122.0, 8.0, 13.0, NEGRO),
        esfera(423.0, 113.0, 8.0, 13.0, NEGRO),
        // Cuerpo: pocas esferas grandes solapadas, para que se noten los bultos
        esfera(300.0, 211.0, 144.0, 10.0, AZUL),
        esfera(300.0, 268.0, 144.0, 10.0, AZUL),
        esfera(300.0, 325.0, 144.0, 10.0, AZUL),
        esfera(300.0, 382.0, 144.0, 10.0, AZUL),
        // Ojos y pupilas
        esfera(237.0, 135.0, 41.0, 3.5, BLANCO),
        esfera(348.0, 131.0, 42.0, 3.5, BLANCO),
        esfera(215.0, 137.0, 13.0, 2.5, NEGRO),
        esfera(326.0, 134.0, 12.5, 2.5, NEGRO),
        // Hocico y nariz
        esfera(283.0, 192.0, 16.0, 3.5, PIEL),
        esfera(314.0, 192.0, 16.0, 3.5, PIEL),
        esfera(296.0, 172.0, 13.0, 2.5, NEGRO),
        // Dientes
        esfera(286.0, 215.0, 9.0, 3.0, BLANCO),
        esfera(305.0, 216.0, 8.0, 3.0, BLANCO),
    ]
}
