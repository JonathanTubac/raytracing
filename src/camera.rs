use nalgebra_glm::{cross, normalize, Vec3};

const DISTANCIA_MIN: f32 = 1.0;
const DISTANCIA_MAX: f32 = 30.0;
// Limite del angulo vertical para no llegar a mirar justo desde arriba/abajo,
// donde la direccion "arriba" de la camara se vuelve indefinida
const PITCH_MAX: f32 = 1.5;

/// Camara orbital: siempre mira a `center` y se mueve sobre una esfera a su alrededor.
pub struct Camera {
    pub center: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Camera {
    pub fn new(center: Vec3, distance: f32) -> Self {
        Camera {
            center,
            distance,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    /// Posicion de la camara en el mundo
    pub fn eye(&self) -> Vec3 {
        let offset = Vec3::new(
            self.pitch.cos() * self.yaw.sin(),
            self.pitch.sin(),
            self.pitch.cos() * self.yaw.cos(),
        );
        self.center + offset * self.distance
    }

    /// Rota la camara alrededor del centro (en radianes)
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-PITCH_MAX, PITCH_MAX);
    }

    /// Acerca (factor < 1) o aleja (factor > 1) la camara del centro
    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(DISTANCIA_MIN, DISTANCIA_MAX);
    }

    /// Convierte un vector del espacio de la camara (mirando hacia -Z) al espacio del mundo
    pub fn basis_change(&self, vector: &Vec3) -> Vec3 {
        let forward = normalize(&(self.center - self.eye()));
        let right = normalize(&cross(&forward, &Vec3::new(0.0, 1.0, 0.0)));
        let up = cross(&right, &forward);

        vector.x * right + vector.y * up - vector.z * forward
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: &Vec3, b: &Vec3) -> bool {
        (a - b).norm() < 1e-5
    }

    #[test]
    fn camara_inicial_queda_delante_del_centro_mirando_a_menos_z() {
        let camera = Camera::new(Vec3::new(0.0, 0.0, -5.0), 5.0);

        assert!(cerca(&camera.eye(), &Vec3::zeros()));
        // Sin rotar, la base de la camara es la del mundo
        assert!(cerca(&camera.basis_change(&Vec3::new(0.0, 0.0, -1.0)), &Vec3::new(0.0, 0.0, -1.0)));
        assert!(cerca(&camera.basis_change(&Vec3::new(1.0, 0.0, 0.0)), &Vec3::new(1.0, 0.0, 0.0)));
        assert!(cerca(&camera.basis_change(&Vec3::new(0.0, 1.0, 0.0)), &Vec3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn orbitar_mantiene_la_distancia_y_sigue_mirando_al_centro() {
        let mut camera = Camera::new(Vec3::new(1.0, 2.0, -5.0), 5.0);
        camera.orbit(0.9, 0.4);

        assert!(((camera.eye() - camera.center).norm() - 5.0).abs() < 1e-4);

        let hacia_el_centro = normalize(&(camera.center - camera.eye()));
        let mirando = camera.basis_change(&Vec3::new(0.0, 0.0, -1.0));
        assert!(cerca(&hacia_el_centro, &mirando));
    }

    #[test]
    fn un_cuarto_de_vuelta_pone_la_camara_a_un_lado() {
        let mut camera = Camera::new(Vec3::zeros(), 4.0);
        camera.orbit(std::f32::consts::FRAC_PI_2, 0.0);

        assert!(cerca(&camera.eye(), &Vec3::new(4.0, 0.0, 0.0)));
    }

    #[test]
    fn el_zoom_y_el_pitch_tienen_limites() {
        let mut camera = Camera::new(Vec3::zeros(), 5.0);

        for _ in 0..200 {
            camera.zoom(0.5);
        }
        assert_eq!(camera.distance, DISTANCIA_MIN);

        for _ in 0..200 {
            camera.zoom(2.0);
        }
        assert_eq!(camera.distance, DISTANCIA_MAX);

        camera.orbit(0.0, 100.0);
        assert_eq!(camera.pitch, PITCH_MAX);
    }
}
