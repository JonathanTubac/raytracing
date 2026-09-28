use std::ops::{Add, AddAssign, Mul};

/// Color con canales `f32`, donde 1.0 es el maximo que puede mostrar la pantalla.
///
/// Mientras se calcula la luz los canales pueden pasar de 1.0 (un brillo muy fuerte, una
/// luz emisiva) sin que se pierda nada; recien al mostrarlo se comprime al rango de la
/// pantalla con `tone_map` y se convierte a bytes con `to_hex`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

// Hasta este valor el tone mapping deja el color igual; por encima lo comprime
const RODILLA: f32 = 0.8;

impl Color {
    /// Color a partir de sus bytes (0 a 255 por canal), como se escriben normalmente
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Color a partir de sus canales en punto flotante (1.0 = maximo de la pantalla)
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b }
    }

    pub const fn black() -> Self {
        Color::rgb(0.0, 0.0, 0.0)
    }

    /// Mezcla dos colores: `t` = 0 da `a`, `t` = 1 da `b`
    pub fn lerp(a: Color, b: Color, t: f32) -> Color {
        a * (1.0 - t) + b * t
    }

    /// Comprime los canales que pasan de `RODILLA` para que se acerquen a 1.0 sin llegar
    /// nunca de golpe. Asi un brillo muy fuerte se ve blanco y suave en vez de un parche
    /// plano saturado, y los colores normales (por debajo de la rodilla) no cambian.
    pub fn tone_map(&self) -> Color {
        let canal = |c: f32| {
            if c <= RODILLA {
                c
            } else {
                let rango = 1.0 - RODILLA;
                RODILLA + rango * (1.0 - (-(c - RODILLA) / rango).exp())
            }
        };
        Color::rgb(canal(self.r), canal(self.g), canal(self.b))
    }

    /// Convierte el color a 0xRRGGBB, que es el formato que usa el framebuffer. Lo que se
    /// sale de 0..1 se recorta.
    pub fn to_hex(&self) -> u32 {
        let byte = |c: f32| (c * 255.0).round().clamp(0.0, 255.0) as u32;
        (byte(self.r) << 16) | (byte(self.g) << 8) | byte(self.b)
    }
}

/// Oscurece (factor < 1) o aclara (factor > 1) un color
impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, factor: f32) -> Color {
        Color::rgb(self.r * factor, self.g * factor, self.b * factor)
    }
}

/// Multiplica canal por canal: una luz de color sobre una superficie de color. Una luz
/// roja sobre una superficie verde da negro.
impl Mul for Color {
    type Output = Color;

    fn mul(self, other: Color) -> Color {
        Color::rgb(self.r * other.r, self.g * other.g, self.b * other.b)
    }
}

/// Suma dos colores; el resultado puede pasar de 1.0
impl Add for Color {
    type Output = Color;

    fn add(self, other: Color) -> Color {
        Color::rgb(self.r + other.r, self.g + other.g, self.b + other.b)
    }
}

impl AddAssign for Color {
    fn add_assign(&mut self, other: Color) {
        *self = *self + other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_bytes_van_y_vuelven_sin_perder_nada() {
        assert_eq!(Color::new(12, 200, 255).to_hex(), 0x0CC8FF);
    }

    #[test]
    fn sumar_no_satura_pero_to_hex_si_recorta() {
        let fuerte = Color::rgb(0.9, 0.0, 0.0) + Color::rgb(0.9, 0.0, 0.0);
        assert!((fuerte.r - 1.8).abs() < 1e-6);
        assert_eq!(fuerte.to_hex(), 0xFF0000);
    }

    #[test]
    fn la_luz_de_color_filtra_la_superficie() {
        let roja = Color::rgb(1.0, 0.0, 0.0);
        let verde = Color::rgb(0.0, 1.0, 0.0);
        assert_eq!((roja * verde).to_hex(), 0x000000);
        assert_eq!((roja * Color::rgb(0.5, 1.0, 1.0)).to_hex(), 0x800000);
    }

    #[test]
    fn el_tone_mapping_respeta_lo_oscuro_y_nunca_pasa_de_uno() {
        let normal = Color::rgb(0.2, 0.5, 0.8);
        assert_eq!(normal.tone_map(), normal);

        let mut anterior = 0.0;
        for i in 1..100 {
            let c = Color::rgb(i as f32 * 0.1, 0.0, 0.0).tone_map().r;
            assert!(c >= anterior && c <= 1.0, "{c} con entrada {}", i as f32 * 0.1);
            anterior = c;
        }
    }
}
