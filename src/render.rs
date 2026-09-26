use nalgebra_glm::{Vec3, normalize};

use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::ray_intersect::RayIntersect;
use crate::sphere::Sphere;

// Color de los pixeles donde el rayo no golpea ninguna esfera
const FONDO: Color = Color::new(20, 20, 30);

pub fn cast_ray(ray_origin: &Vec3, ray_direction: &Vec3, objects: &[Sphere]) -> Color {
    // Z-buffer: guarda la distancia del impacto mas cercano visto hasta ahora en este rayo.
    // Un objeto solo se pinta si choca mas cerca que lo que ya esta guardado, asi el
    // resultado no depende del orden en que se agregaron las esferas.
    let mut zbuffer = f32::INFINITY;
    let mut color = FONDO;

    for object in objects {
        let intersect = object.ray_intersect(ray_origin, ray_direction);
        if intersect.is_intersecting && intersect.distance < zbuffer {
            zbuffer = intersect.distance;
            color = intersect.material.diffuse;
        }
    }

    color
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Sphere]) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;

    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            // Mapea el pixel a espacio de pantalla [-1, 1]
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            // Ajuste por aspect ratio
            let screen_x = screen_x * aspect_ratio;

            // Direccion del rayo para este pixel
            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));

            // Se lanza el rayo y se obtiene el color
            let pixel_color = cast_ray(&Vec3::new(0.0, 0.0, 0.0), &ray_direction, objects);

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

    fn lanzar(objects: &[Sphere]) -> Color {
        cast_ray(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0), objects)
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
        let color = cast_ray(&Vec3::zeros(), &Vec3::new(1.0, 0.0, 0.0), &objects);
        assert_eq!(color.to_hex(), FONDO.to_hex());
    }
}
