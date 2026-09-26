use nalgebra_glm::{dot, normalize, Vec3};

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::sphere::Sphere;

// Color de los pixeles donde el rayo no golpea ninguna esfera
const FONDO: Color = Color::new(20, 20, 30);

// Luz minima que recibe cualquier superficie, aunque la luz no le llegue de frente
const AMBIENTE: f32 = 0.15;

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Sphere],
    light: &Light,
) -> Color {
    // Z-buffer: guarda la distancia del impacto mas cercano visto hasta ahora en este rayo.
    // Un objeto solo se pinta si choca mas cerca que lo que ya esta guardado, asi el
    // resultado no depende del orden en que se agregaron las esferas.
    let mut zbuffer = f32::INFINITY;
    let mut closest: Option<Intersect> = None;

    for object in objects {
        let intersect = object.ray_intersect(ray_origin, ray_direction);
        if intersect.is_intersecting && intersect.distance < zbuffer {
            zbuffer = intersect.distance;
            closest = Some(intersect);
        }
    }

    match closest {
        Some(hit) => {
            // Luz difusa: entre mas de frente le llega la luz a la superficie, mas brillante.
            // Es el coseno del angulo entre la normal y la direccion hacia la luz.
            let light_dir = normalize(&(light.position - hit.point));
            let difuso = dot(&hit.normal, &light_dir).max(0.0) * light.intensity;

            hit.material.diffuse * (AMBIENTE + difuso).min(1.0)
        }
        None => FONDO,
    }
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Sphere], camera: &Camera, light: &Light) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let eye = camera.eye();

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            // Mapea el pixel a espacio de pantalla [-1, 1]
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            // Ajuste por aspect ratio
            let screen_x = screen_x * aspect_ratio;

            // Direccion del rayo en el espacio de la camara, y luego en el del mundo
            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
            let ray_direction = camera.basis_change(&ray_direction);

            // Se lanza el rayo desde la camara y se obtiene el color
            let pixel_color = cast_ray(&eye, &ray_direction, objects, light);

            framebuffer.set_current_color(pixel_color.to_hex());
            framebuffer.point(x, y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ray_intersect::Material;

    // Esfera en el eje Z cuyo color rojo identifica cual es: `id` es el valor del canal r
    fn esfera(z: f32, id: u8) -> Sphere {
        Sphere {
            center: Vec3::new(0.0, 0.0, z),
            radius: 1.0,
            material: Material {
                diffuse: Color::new(id, 0, 0),
            },
        }
    }

    // Luz en el origen: le pega de frente a todo lo que esta sobre el eje Z, asi los
    // colores de estas pruebas salen sin oscurecer
    fn luz_de_frente() -> Light {
        Light::new(Vec3::zeros(), 1.0)
    }

    fn lanzar(objects: &[Sphere]) -> Color {
        cast_ray(
            &Vec3::zeros(),
            &Vec3::new(0.0, 0.0, -1.0),
            objects,
            &luz_de_frente(),
        )
    }

    #[test]
    fn pinta_la_esfera_mas_cercana_sin_importar_el_orden() {
        let cerca = esfera(-3.0, 255);
        let lejos = esfera(-5.0, 100);

        // Cerca agregada primero, y despues al reves
        assert_eq!(lanzar(&[cerca, lejos]).r, 255);
        assert_eq!(lanzar(&[esfera(-5.0, 100), esfera(-3.0, 255)]).r, 255);
    }

    #[test]
    fn con_tres_esferas_gana_la_del_medio_si_es_la_mas_cercana() {
        let objects = [esfera(-6.0, 1), esfera(-2.0, 2), esfera(-4.0, 3)];
        assert_eq!(lanzar(&objects).r, 2);
    }

    #[test]
    fn ignora_esferas_detras_de_la_camara() {
        let objects = [esfera(4.0, 200), esfera(-5.0, 100)];
        assert_eq!(lanzar(&objects).r, 100);
    }

    #[test]
    fn si_no_hay_impacto_devuelve_el_fondo() {
        let objects = [esfera(-5.0, 100)];
        let color = cast_ray(
            &Vec3::zeros(),
            &Vec3::new(1.0, 0.0, 0.0),
            &objects,
            &luz_de_frente(),
        );
        assert_eq!(color.to_hex(), FONDO.to_hex());
    }

    #[test]
    fn la_cara_que_mira_a_la_luz_es_mas_clara_que_la_que_le_da_la_espalda() {
        let objects = [esfera(-5.0, 200)];
        let origen = Vec3::zeros();
        let direccion = Vec3::new(0.0, 0.0, -1.0);

        let luz_delante = Light::new(Vec3::zeros(), 1.0);
        let luz_detras = Light::new(Vec3::new(0.0, 0.0, -10.0), 1.0);

        let iluminada = cast_ray(&origen, &direccion, &objects, &luz_delante);
        let en_sombra = cast_ray(&origen, &direccion, &objects, &luz_detras);

        assert_eq!(iluminada.r, 200);
        // Sin luz directa solo queda la luz ambiente
        assert_eq!(en_sombra.r, (200.0 * AMBIENTE).round() as u8);
    }
}
