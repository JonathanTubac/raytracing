use crate::camera::Camera;
use crate::window::{Key, Window};

// How much the camera rotates (radians) per frame while an arrow key is held
const ROTACION_TECLADO: f32 = 0.03;
// How much the camera rotates (radians) per pixel the mouse is dragged
const ROTACION_MOUSE: f32 = 0.01;
// Zoom factor per frame while W/S is held (less than 1 = zoom in)
const ZOOM_TECLADO: f32 = 0.97;
// How much each mouse wheel "click" zooms in or out
const ZOOM_RUEDA: f32 = 0.1;

/// Reads the keyboard and mouse and moves the camera
pub struct Controls {
    last_mouse: Option<(f32, f32)>,
}

impl Controls {
    pub fn new() -> Self {
        Controls { last_mouse: None }
    }

    /// Updates the camera from what is being pressed
    pub fn update(&mut self, window: &Window, camera: &mut Camera) -> bool {
        let mut moved = false;

        // Rotate with the keyboard
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

        // Rotate by dragging the mouse: the scene follows the cursor
        let mouse = window.mouse_pos();
        if window.is_mouse_down() {
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

        // Zoom with the keyboard and the wheel
        if window.is_key_down(Key::W) {
            camera.zoom(ZOOM_TECLADO);
            moved = true;
        }
        if window.is_key_down(Key::S) {
            camera.zoom(1.0 / ZOOM_TECLADO);
            moved = true;
        }
        let scroll = window.scroll_wheel();
        if scroll != 0.0 {
            camera.zoom((1.0 - ZOOM_RUEDA * scroll).clamp(0.5, 1.5));
            moved = true;
        }

        moved
    }
}
