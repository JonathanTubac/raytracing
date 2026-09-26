use minifb::{Key, MouseButton, MouseMode, Window};

use crate::camera::Camera;

// Cuanto rota la camara (radianes) por cada frame con una flecha apretada
const ROTACION_TECLADO: f32 = 0.03;
// Cuanto rota la camara (radianes) por cada pixel que se arrastra el mouse
const ROTACION_MOUSE: f32 = 0.01;
// Factor de zoom por frame con W/S apretada (menor que 1 = acercar)
const ZOOM_TECLADO: f32 = 0.97;
// Cuanto acerca/aleja cada "click" de la rueda del mouse
const ZOOM_RUEDA: f32 = 0.1;

/// Lee el teclado y el mouse y mueve la camara.
///
/// - Flechas o arrastrar con click izquierdo: rotar alrededor de las esferas
/// - W / S o rueda del mouse: acercar / alejar
pub struct Controls {
    last_mouse: Option<(f32, f32)>,
}

impl Controls {
    pub fn new() -> Self {
        Controls { last_mouse: None }
    }

    /// Actualiza la camara segun lo que se esta presionando. Devuelve `true` si la
    /// camara se movio, o sea si hay que volver a renderizar.
    pub fn update(&mut self, window: &Window, camera: &mut Camera) -> bool {
        let mut moved = false;

        // Rotar con el teclado. Izquierda/derecha giran la camara hacia ese lado,
        // arriba/abajo la suben o la bajan
        let mut yaw = 0.0;
        let mut pitch = 0.0;
        if window.is_key_down(Key::Left) {
            yaw -= ROTACION_TECLADO;
        }
        if window.is_key_down(Key::Right) {
            yaw += ROTACION_TECLADO;
        }
        if window.is_key_down(Key::Up) {
            pitch += ROTACION_TECLADO;
        }
        if window.is_key_down(Key::Down) {
            pitch -= ROTACION_TECLADO;
        }

        // Rotar arrastrando con el mouse: la escena sigue al cursor
        let mouse = window.get_mouse_pos(MouseMode::Discard);
        if window.get_mouse_down(MouseButton::Left) {
            if let (Some((x, y)), Some((last_x, last_y))) = (mouse, self.last_mouse) {
                yaw -= (x - last_x) * ROTACION_MOUSE;
                pitch += (y - last_y) * ROTACION_MOUSE;
            }
        }
        self.last_mouse = mouse;

        if yaw != 0.0 || pitch != 0.0 {
            camera.orbit(yaw, pitch);
            moved = true;
        }

        // Zoom con el teclado y con la rueda
        if window.is_key_down(Key::W) {
            camera.zoom(ZOOM_TECLADO);
            moved = true;
        }
        if window.is_key_down(Key::S) {
            camera.zoom(1.0 / ZOOM_TECLADO);
            moved = true;
        }
        if let Some((_, scroll)) = window.get_scroll_wheel() {
            if scroll != 0.0 {
                camera.zoom((1.0 - ZOOM_RUEDA * scroll).clamp(0.5, 1.5));
                moved = true;
            }
        }

        moved
    }
}
