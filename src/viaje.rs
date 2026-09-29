//! El viaje por el portal: la camara vuela a traves de la puerta de la cabana hasta el
//! portal, la imagen se funde en el remolino morado del portal, aparece del otro lado
//! dentro del portal de llegada y se aleja hasta la vista del nuevo mundo.
//!
//! Son cuatro etapas. En cada una la camara va de una pose a otra con una curva suave
//! (arranca y frena despacio), asi el movimiento no se ve mecanico.

use std::f32::consts::{PI, TAU};

use crate::camera::Camera;
use crate::math::Vec3;

// Duracion de cada etapa, en segundos
const ACERCARSE: f32 = 1.4;
const ENTRAR: f32 = 1.1;
const SALIR: f32 = 0.9;
const ALEJARSE: f32 = 1.6;

// Frente al portal, antes de entrar: a esta distancia, un poco desde arriba
const DISTANCIA_FRENTE: f32 = 7.0;
const ALTURA_FRENTE: f32 = 0.06;
// Distancia al centro del portal cuando se lo cruza: la camara queda dentro del portal
const DISTANCIA_DENTRO: f32 = 0.3;

/// Lo que se interpola de la camara durante el viaje
#[derive(Clone, Copy)]
struct Pose {
    center: Vec3,
    distance: f32,
    yaw: f32,
    pitch: f32,
}

impl Pose {
    fn of(camera: &Camera) -> Pose {
        Pose {
            center: camera.center,
            distance: camera.distance,
            yaw: camera.yaw,
            pitch: camera.pitch,
        }
    }

    /// Mirando el portal de frente. Los dos portales se abren hacia +Z, que es la
    /// direccion en que la camara mira con yaw = 0.
    fn facing(portal: Vec3, distance: f32) -> Pose {
        Pose {
            center: portal,
            distance,
            yaw: 0.0,
            pitch: ALTURA_FRENTE,
        }
    }

    fn apply(&self, camera: &mut Camera) {
        camera.center = self.center;
        camera.distance = self.distance;
        camera.yaw = self.yaw;
        camera.pitch = self.pitch;
    }

    /// Pose intermedia: `t` = 0 da `a` y `t` = 1 da `b`. El angulo horizontal gira por el
    /// lado mas corto, para no dar una vuelta entera si la camara ya habia girado mucho.
    fn lerp(a: &Pose, b: &Pose, t: f32) -> Pose {
        let mix = |x: f32, y: f32| x + (y - x) * t;
        let yaw_diff = (b.yaw - a.yaw + PI).rem_euclid(TAU) - PI;
        Pose {
            center: a.center + (b.center - a.center) * t,
            distance: mix(a.distance, b.distance),
            yaw: a.yaw + yaw_diff * t,
            pitch: mix(a.pitch, b.pitch),
        }
    }
}

/// Curva suave de 0 a 1: arranca y termina despacio
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Etapa {
    Acercarse,
    Entrar,
    Salir,
    Alejarse,
}

/// Que hacer despues de avanzar el viaje
#[derive(PartialEq, Debug)]
pub enum Paso {
    Sigue,
    /// La camara cruzo el portal: hay que cambiar al otro mundo
    CambiarMundo,
    Termino,
}

pub struct Viaje {
    etapa: Etapa,
    /// Segundos desde que empezo la etapa
    tiempo: f32,
    /// Pose de la camara al empezar el viaje
    desde: Pose,
}

impl Viaje {
    pub fn new(camera: &Camera) -> Viaje {
        Viaje {
            etapa: Etapa::Acercarse,
            tiempo: 0.0,
            desde: Pose::of(camera),
        }
    }

    /// Avanza `dt` segundos y mueve la camara. `portal` es el centro del portal del mundo
    /// en el que se esta (despues de `CambiarMundo`, el del mundo nuevo) y `home` la vista
    /// inicial de ese mundo.
    pub fn avanzar(&mut self, dt: f32, camera: &mut Camera, portal: Vec3, home: &Camera) -> Paso {
        self.tiempo += dt;
        let frente = Pose::facing(portal, DISTANCIA_FRENTE);
        let dentro = Pose::facing(portal, DISTANCIA_DENTRO);

        let (desde, hasta, duracion) = match self.etapa {
            Etapa::Acercarse => (self.desde, frente, ACERCARSE),
            Etapa::Entrar => (frente, dentro, ENTRAR),
            Etapa::Salir => (dentro, frente, SALIR),
            Etapa::Alejarse => (frente, Pose::of(home), ALEJARSE),
        };
        let t = self.tiempo / duracion;
        Pose::lerp(&desde, &hasta, ease(t)).apply(camera);

        if t < 1.0 {
            return Paso::Sigue;
        }

        // Termino la etapa: se pasa a la siguiente
        self.tiempo = 0.0;
        match self.etapa {
            Etapa::Acercarse => {
                self.etapa = Etapa::Entrar;
                Paso::Sigue
            }
            Etapa::Entrar => {
                self.etapa = Etapa::Salir;
                Paso::CambiarMundo
            }
            Etapa::Salir => {
                self.etapa = Etapa::Alejarse;
                Paso::Sigue
            }
            Etapa::Alejarse => Paso::Termino,
        }
    }

    /// Cuanto tapa la pantalla el remolino morado (0 = nada, 1 = todo): aparece en la
    /// segunda mitad de la entrada y se va en la primera parte de la salida
    pub fn overlay(&self) -> f32 {
        match self.etapa {
            Etapa::Entrar => ease((self.tiempo / ENTRAR - 0.4) / 0.6),
            Etapa::Salir => 1.0 - ease(self.tiempo / SALIR / 0.7),
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn el_viaje_cruza_el_portal_una_vez_y_termina_en_la_vista_del_otro_mundo() {
        let portal_a = Vec3::new(-6.5, 2.0, -1.0);
        let portal_b = Vec3::new(-7.5, 2.0, 3.0);
        let mut home = Camera::new(Vec3::new(0.0, -3.0, 0.0), 24.0);
        home.orbit(0.3, 0.4);

        let mut camera = home;
        camera.orbit(2.0, 0.1);
        let mut viaje = Viaje::new(&camera);

        let mut portal = portal_a;
        let mut cambios = 0;
        let mut dentro_del_portal = false;
        for _ in 0..1000 {
            match viaje.avanzar(1.0 / 60.0, &mut camera, portal, &home) {
                Paso::Sigue => {}
                Paso::CambiarMundo => {
                    cambios += 1;
                    // Justo al cruzar, la camara esta dentro del portal y la pantalla morada
                    dentro_del_portal = (camera.eye() - portal).norm() < 0.5;
                    portal = portal_b;
                }
                Paso::Termino => break,
            }
        }

        assert_eq!(cambios, 1);
        assert!(dentro_del_portal);
        assert!((camera.center - home.center).norm() < 1e-4);
        assert!(cerca(camera.distance, home.distance));
        assert!(cerca(camera.pitch, home.pitch));
        // Mismo angulo, aunque haya dado vueltas de mas o de menos
        assert!(cerca(camera.yaw.sin(), home.yaw.sin()) && cerca(camera.yaw.cos(), home.yaw.cos()));
    }

    #[test]
    fn el_morado_aparece_al_entrar_y_se_va_al_salir() {
        let mut viaje = Viaje::new(&Camera::new(Vec3::zeros(), 10.0));
        assert_eq!(viaje.overlay(), 0.0);

        viaje.etapa = Etapa::Entrar;
        viaje.tiempo = ENTRAR;
        assert!(cerca(viaje.overlay(), 1.0));

        viaje.etapa = Etapa::Salir;
        viaje.tiempo = 0.0;
        assert!(cerca(viaje.overlay(), 1.0));
        viaje.tiempo = SALIR;
        assert!(cerca(viaje.overlay(), 0.0));
    }

    #[test]
    fn el_giro_va_por_el_lado_mas_corto() {
        let a = Pose::facing(Vec3::zeros(), 1.0);
        let b = Pose { yaw: TAU - 0.2, ..a };
        // De 0 a casi una vuelta completa, la mitad del camino es -0.1 y no casi media vuelta
        assert!(cerca(Pose::lerp(&a, &b, 0.5).yaw, -0.1));
    }
}
