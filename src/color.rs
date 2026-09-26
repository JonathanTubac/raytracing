use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Mezcla dos colores: `t` = 0 da `a`, `t` = 1 da `b`
    pub fn lerp(a: Color, b: Color, t: f32) -> Color {
        a * (1.0 - t) + b * t
    }

    /// Convierte el color a 0xRRGGBB, que es el formato que usa el framebuffer
    pub fn to_hex(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

/// Oscurece (factor < 1) o aclara (factor > 1) un color, sin salirse de 0..=255
impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, factor: f32) -> Color {
        let canal = |c: u8| (c as f32 * factor).round().clamp(0.0, 255.0) as u8;
        Color::new(canal(self.r), canal(self.g), canal(self.b))
    }
}

/// Suma dos colores, sin pasarse de 255 en ningun canal
impl Add for Color {
    type Output = Color;

    fn add(self, other: Color) -> Color {
        Color::new(
            self.r.saturating_add(other.r),
            self.g.saturating_add(other.g),
            self.b.saturating_add(other.b),
        )
    }
}
