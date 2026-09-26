use nalgebra_glm::Vec3;

/// Luz puntual: ilumina desde `position` con una fuerza `intensity` (1.0 = normal)
pub struct Light {
    pub position: Vec3,
    pub intensity: f32,
}

impl Light {
    pub fn new(position: Vec3, intensity: f32) -> Self {
        Light {
            position,
            intensity,
        }
    }
}
