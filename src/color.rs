use std::ops::{Add, AddAssign, Mul};

/// Color with `f32` channels, where 1.0 is the brightest the screen can show
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

// Tone mapping leaves colors up to this value unchanged and compresses above it
const RODILLA: f32 = 0.8;

impl Color {
    /// Color from its bytes (0 to 255 per channel), as they are usually written
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Color from floating point channels (1.0 = screen maximum)
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Color { r, g, b }
    }

    pub const fn black() -> Self {
        Color::rgb(0.0, 0.0, 0.0)
    }

    /// Blends two colors: `t` = 0 gives `a`, `t` = 1 gives `b`
    pub fn lerp(a: Color, b: Color, t: f32) -> Color {
        a * (1.0 - t) + b * t
    }

    /// Smoothly compresses channels above `RODILLA` toward 1.0
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

    /// Converts the color to 0xRRGGBB, the framebuffer format
    pub fn to_hex(&self) -> u32 {
        let byte = |c: f32| (c * 255.0).round().clamp(0.0, 255.0) as u32;
        (byte(self.r) << 16) | (byte(self.g) << 8) | byte(self.b)
    }
}

/// Darkens (factor < 1) or brightens (factor > 1) a color
impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, factor: f32) -> Color {
        Color::rgb(self.r * factor, self.g * factor, self.b * factor)
    }
}

/// Multiplies channel by channel: a colored light on a colored surface
impl Mul for Color {
    type Output = Color;

    fn mul(self, other: Color) -> Color {
        Color::rgb(self.r * other.r, self.g * other.g, self.b * other.b)
    }
}

/// Adds two colors; the result can go above 1.0
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
