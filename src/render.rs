use nalgebra_glm::{dot, normalize, Vec3};
use rayon::prelude::*;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::sphere::Sphere;

// Color del cielo abajo (horizonte hacia el suelo) y arriba. Los rayos que no golpean nada,
// incluidos los que salen de reflejos y refracciones, toman su color de este degradado.
const FONDO_ABAJO: Color = Color::new(25, 25, 40);
const FONDO_ARRIBA: Color = Color::new(110, 140, 200);

// Color de la luz, usado en el brillo especular
const BLANCO: Color = Color::new(255, 255, 255);

// Luz minima que recibe cualquier superficie, aunque la luz no le llegue de frente
const AMBIENTE: f32 = 0.15;

// Cuantas veces puede rebotar o atravesar un rayo antes de rendirse
const MAX_DEPTH: u32 = 4;

// Separacion al lanzar rayos nuevos desde una superficie, para que no choquen con ella misma
const BIAS: f32 = 1e-3;

/// Color del fondo en una direccion: un degradado vertical
fn fondo(direction: &Vec3) -> Color {
    let t = (0.5 * (direction.y + 1.0)).clamp(0.0, 1.0);
    Color::lerp(FONDO_ABAJO, FONDO_ARRIBA, t)
}

/// Direccion en que rebota un rayo que llega con `incident` a una superficie con esa normal
fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * dot(incident, normal) * normal
}

/// Direccion en que se dobla un rayo al atravesar la superficie (ley de Snell).
/// Devuelve `None` en reflexion total interna: cuando el rayo sale de un material denso
/// con un angulo tan cerrado que no puede salir y rebota por dentro.
fn refract(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> Option<Vec3> {
    let mut cos_i = -dot(incident, normal).clamp(-1.0, 1.0);
    let mut normal = *normal;
    let (mut eta_i, mut eta_t) = (1.0, refractive_index);

    // Si el rayo ya esta dentro del material (la normal apunta hacia afuera y el rayo
    // tambien), se invierte todo: sale del material hacia el aire
    if cos_i < 0.0 {
        cos_i = -cos_i;
        normal = -normal;
        std::mem::swap(&mut eta_i, &mut eta_t);
    }

    let eta = eta_i / eta_t;
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);

    if k < 0.0 {
        None
    } else {
        Some(eta * incident + (eta * cos_i - k.sqrt()) * normal)
    }
}

/// Punto desde donde lanzar un rayo nuevo: apenas separado de la superficie, del lado
/// hacia donde va el rayo
fn offset_origin(point: &Vec3, direction: &Vec3, normal: &Vec3) -> Vec3 {
    if dot(direction, normal) < 0.0 {
        point - normal * BIAS
    } else {
        point + normal * BIAS
    }
}

pub fn cast_ray(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[Sphere],
    light: &Light,
    depth: u32,
) -> Color {
    if depth > MAX_DEPTH {
        return fondo(ray_direction);
    }

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

    let Some(hit) = closest else {
        return fondo(ray_direction);
    };
    let material = hit.material;

    // Luz difusa: entre mas de frente le llega la luz a la superficie, mas brillante.
    // Es el coseno del angulo entre la normal y la direccion hacia la luz. El color
    // viene de la textura del material en el punto de impacto.
    let light_dir = normalize(&(light.position - hit.point));
    let diffuse_intensity = dot(&hit.normal, &light_dir).max(0.0) * light.intensity;
    let base = material.texture.color_at(hit.u, hit.v);
    let diffuse = base * ((AMBIENTE + diffuse_intensity).min(1.0) * material.albedo[0]);

    // Luz especular: el brillo de la luz sobre la superficie, que se ve solo cuando la
    // camara esta cerca de la direccion en la que rebota la luz
    let view_dir = normalize(&-ray_direction);
    let reflected_light = reflect(&-light_dir, &hit.normal);
    let specular_intensity =
        dot(&view_dir, &reflected_light).max(0.0).powf(material.specular) * light.intensity;
    let specular = BLANCO * (specular_intensity * material.albedo[1]);

    let mut color = diffuse + specular;

    // Reflejo: se lanza otro rayo en la direccion de rebote y se mezcla lo que encuentre
    if material.reflectivity > 0.0 {
        let direction = normalize(&reflect(ray_direction, &hit.normal));
        let origin = offset_origin(&hit.point, &direction, &hit.normal);
        let reflected = cast_ray(&origin, &direction, objects, light, depth + 1);

        color = color + reflected * material.reflectivity;
    }

    // Transparencia: se lanza otro rayo que atraviesa la superficie doblandose. Si no puede
    // atravesarla (reflexion total interna), el rayo rebota por dentro.
    if material.transparency > 0.0 {
        let direction = match refract(ray_direction, &hit.normal, material.refractive_index) {
            Some(refracted) => normalize(&refracted),
            None => normalize(&reflect(ray_direction, &hit.normal)),
        };
        let origin = offset_origin(&hit.point, &direction, &hit.normal);
        let refracted = cast_ray(&origin, &direction, objects, light, depth + 1);

        color = color + refracted * material.transparency;
    }

    color
}

pub fn render(framebuffer: &mut Framebuffer, objects: &[Sphere], camera: &Camera, light: &Light) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let eye = camera.eye();

    let pixels_wide = framebuffer.width;

    // Cada pixel es independiente de los demas, asi que los colores se calculan en
    // paralelo entre todos los nucleos del procesador
    let colors: Vec<u32> = (0..framebuffer.width * framebuffer.height)
        .into_par_iter()
        .map(|i| {
            let x = i % pixels_wide;
            let y = i / pixels_wide;

            // Mapea el pixel a espacio de pantalla [-1, 1]
            let screen_x = (2.0 * x as f32) / width - 1.0;
            let screen_y = -(2.0 * y as f32) / height + 1.0;

            // Ajuste por aspect ratio
            let screen_x = screen_x * aspect_ratio;

            // Direccion del rayo en el espacio de la camara, y luego en el del mundo
            let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
            let ray_direction = camera.basis_change(&ray_direction);

            // Se lanza el rayo desde la camara y se obtiene el color
            cast_ray(&eye, &ray_direction, objects, light, 0).to_hex()
        })
        .collect();

    // Pintar el framebuffer es barato, se hace en orden
    for (i, color) in colors.into_iter().enumerate() {
        framebuffer.set_current_color(color);
        framebuffer.point(i % pixels_wide, i / pixels_wide);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ray_intersect::Material;
    use crate::texture::Texture;

    // Material mate del color dado
    fn mate(color: Color) -> Material {
        Material::new(Texture::Solid(color))
    }

    // Esfera en el eje Z cuyo color rojo identifica cual es: `id` es el valor del canal r
    fn esfera(z: f32, id: u8) -> Sphere {
        Sphere {
            center: Vec3::new(0.0, 0.0, z),
            radius: 1.0,
            material: mate(Color::new(id, 0, 0)),
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
            0,
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
        let direccion = Vec3::new(1.0, 0.0, 0.0);
        let color = cast_ray(&Vec3::zeros(), &direccion, &objects, &luz_de_frente(), 0);
        assert_eq!(color.to_hex(), fondo(&direccion).to_hex());
    }

    #[test]
    fn la_cara_que_mira_a_la_luz_es_mas_clara_que_la_que_le_da_la_espalda() {
        let objects = [esfera(-5.0, 200)];
        let origen = Vec3::zeros();
        let direccion = Vec3::new(0.0, 0.0, -1.0);

        let luz_delante = Light::new(Vec3::zeros(), 1.0);
        let luz_detras = Light::new(Vec3::new(0.0, 0.0, -10.0), 1.0);

        let iluminada = cast_ray(&origen, &direccion, &objects, &luz_delante, 0);
        let en_sombra = cast_ray(&origen, &direccion, &objects, &luz_detras, 0);

        assert_eq!(iluminada.r, 200);
        // Sin luz directa solo queda la luz ambiente
        assert_eq!(en_sombra.r, (200.0 * AMBIENTE).round() as u8);
    }

    #[test]
    fn el_albedo_difuso_escala_el_color_del_material() {
        let mut esfera = esfera(-5.0, 200);
        esfera.material.albedo = [0.5, 0.0];

        assert_eq!(lanzar(&[esfera]).r, 100);
    }

    #[test]
    fn el_color_sale_de_la_textura_en_el_punto_de_impacto() {
        // Ajedrez de rojo y azul: el punto de impacto de frente cae en u = 0.75, v = 0.5
        let a = Color::new(200, 0, 0);
        let b = Color::new(0, 0, 200);
        let textura = Texture::Checker { a, b, tiles: 4 };
        let esfera = Sphere {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.0,
            material: Material::new(textura),
        };

        let esperado = textura.color_at(0.75, 0.5);
        assert_eq!(lanzar(&[esfera]).to_hex(), esperado.to_hex());
    }

    #[test]
    fn el_brillo_especular_solo_aparece_si_el_material_lo_pide() {
        let mate_negro = |albedo_especular: f32| Sphere {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.0,
            material: Material {
                albedo: [0.0, albedo_especular],
                ..mate(Color::new(0, 0, 0))
            },
        };

        // La luz esta justo en la camara, asi que el brillo cae en el centro de la esfera
        assert_eq!(lanzar(&[mate_negro(0.0)]).r, 0);
        assert_eq!(lanzar(&[mate_negro(1.0)]).r, 255);
        assert_eq!(lanzar(&[mate_negro(0.5)]).r, 128);
    }

    #[test]
    fn un_espejo_muestra_lo_que_tiene_a_su_espalda() {
        let espejo = Sphere {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.0,
            material: Material {
                albedo: [0.0, 0.0],
                reflectivity: 1.0,
                ..mate(Color::new(255, 255, 255))
            },
        };
        // Esfera roja detras de la camara: solo se puede ver a traves del reflejo. La cara
        // que se ve mira hacia la camara, que es donde esta la luz.
        let roja = esfera(5.0, 200);

        assert_eq!(lanzar(&[espejo, roja]).r, 200);
    }

    #[test]
    fn el_vidrio_deja_ver_lo_que_tiene_detras() {
        let vidrio = Sphere {
            center: Vec3::new(0.0, 0.0, -3.0),
            radius: 1.0,
            material: Material {
                albedo: [0.0, 0.0],
                transparency: 1.0,
                refractive_index: 1.5,
                ..mate(Color::new(255, 255, 255))
            },
        };
        let roja = esfera(-8.0, 200);

        // De frente el rayo atraviesa el vidrio sin desviarse y llega a la esfera roja
        assert_eq!(lanzar(&[vidrio, roja]).r, 200);
    }

    #[test]
    fn la_refraccion_no_desvia_un_rayo_de_frente_y_si_uno_inclinado() {
        let normal = Vec3::new(0.0, 0.0, 1.0);

        let de_frente = refract(&Vec3::new(0.0, 0.0, -1.0), &normal, 1.5).unwrap();
        assert!((de_frente - Vec3::new(0.0, 0.0, -1.0)).norm() < 1e-5);

        // Al entrar a un material mas denso el rayo se acerca a la normal
        let entrante = normalize(&Vec3::new(1.0, 0.0, -1.0));
        let doblado = refract(&entrante, &normal, 1.5).unwrap();
        assert!(doblado.x.abs() < entrante.x.abs());
    }

    #[test]
    fn hay_reflexion_total_interna_al_salir_con_un_angulo_muy_cerrado() {
        // Rayo dentro del vidrio (va en la misma direccion que la normal) muy inclinado
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let rasante = normalize(&Vec3::new(1.0, 0.0, 0.3));

        assert!(refract(&rasante, &normal, 1.5).is_none());
    }
}
