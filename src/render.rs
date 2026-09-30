use std::sync::Mutex;
use std::thread;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::math::{dot, normalize, Vec3};
use crate::ray_intersect::{Intersect, RayIntersect};
use crate::skybox::Skybox;

// How many times a ray can bounce or pass through before giving up
const MAX_DEPTH: u32 = 4;

// Offset for new rays leaving a surface, so they don't hit the surface itself
const BIAS: f32 = 1e-3;

// Maximum surfaces a shadow ray crosses before treating the light as blocked
const MAX_SHADOW_STEPS: u32 = 16;

/// Scene objects, lights and sky
pub struct Scene<'a, O> {
    pub objects: &'a [O],
    pub lights: &'a [Light],
    pub skybox: &'a Skybox,
    /// Minimum light any surface receives
    pub ambient: Color,
    /// Whether normal maps are used
    pub normal_maps: bool,
}

/// Direction a ray arriving as `incident` bounces off a surface with that normal
fn reflect(incident: &Vec3, normal: &Vec3) -> Vec3 {
    incident - 2.0 * dot(incident, normal) * normal
}

/// Direction a ray bends to when passing through the surface (Snell's law)
fn refract(incident: &Vec3, normal: &Vec3, refractive_index: f32) -> Option<Vec3> {
    let mut cos_i = -dot(incident, normal).clamp(-1.0, 1.0);
    let mut normal = *normal;
    let (mut eta_i, mut eta_t) = (1.0, refractive_index);

    // Ray inside the material: flip everything, it exits into the air
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

/// Origin of a new ray, slightly off the surface
fn offset_origin(point: &Vec3, direction: &Vec3, normal: &Vec3) -> Vec3 {
    if dot(direction, normal) < 0.0 {
        point - normal * BIAS
    } else {
        point + normal * BIAS
    }
}

/// The ray's nearest hit among all scene objects
fn closest_hit<O: RayIntersect>(
    objects: &[O],
    origin: &Vec3,
    direction: &Vec3,
) -> Option<Intersect> {
    // Z-buffer: keeps the distance of the nearest hit seen so far along this ray
    let mut zbuffer = f32::INFINITY;
    let mut closest = None;

    for object in objects {
        let intersect = object.ray_intersect(origin, direction);
        if intersect.is_intersecting && intersect.distance < zbuffer {
            zbuffer = intersect.distance;
            closest = Some(intersect);
        }
    }
    closest
}

/// How much light from `light` reaches `point` (0 = blocked, 1 = all of it)
fn light_visibility<O: RayIntersect>(
    point: &Vec3,
    normal: &Vec3,
    light: &Light,
    objects: &[O],
) -> f32 {
    let to_light = light.position - point;
    let mut remaining = to_light.norm();
    let direction = to_light * (1.0 / remaining);
    let mut origin = offset_origin(point, &direction, normal);

    // Step from surface to surface toward the light
    let mut visibility = 1.0;
    for _ in 0..MAX_SHADOW_STEPS {
        let Some(hit) = closest_hit(objects, &origin, &direction) else {
            return visibility;
        };
        // Reached the light: either passed it or hit the block emitting it
        if hit.distance >= remaining || light.contains(&hit.point) {
            return visibility;
        }

        // All the light goes through leaf gaps; none through the glass frame
        let (_, alpha) = hit.material.texture.sample(hit.u, hit.v, hit.face);
        if !hit.material.is_hole(alpha) {
            visibility *= hit.material.transparency_at(alpha);
            if visibility <= 0.0 {
                return 0.0;
            }
        }

        remaining -= hit.distance;
        origin = offset_origin(&hit.point, &direction, &hit.normal);
    }
    // Too many surfaces in the way: treat the light as blocked
    0.0
}

pub fn cast_ray<O: RayIntersect>(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    scene: &Scene<O>,
    depth: u32,
) -> Color {
    if depth > MAX_DEPTH {
        return scene.skybox.sample(ray_direction);
    }

    // If nothing is hit, the sky in that direction is seen
    let Some(mut hit) = closest_hit(scene.objects, ray_origin, ray_direction) else {
        return scene.skybox.sample(ray_direction);
    };
    let material = hit.material;

    let (base, alpha) = material.texture.sample(hit.u, hit.v, hit.face);

    // Hole in the texture (between leaves): the ray keeps going from there
    if material.is_hole(alpha) {
        let origin = offset_origin(&hit.point, ray_direction, &hit.normal);
        return cast_ray(&origin, ray_direction, scene, depth);
    }

    // Face seen from inside an opaque block: light it toward the camera
    if material.transparency == 0.0 && dot(&hit.normal, ray_direction) > 0.0 {
        hit.normal = -hit.normal;
    }

    // Normal for lighting: the normal map one if there is one, else the face normal
    let normal = match material.normal_map {
        Some(normal_map) if scene.normal_maps => normal_map.perturb(&hit),
        _ => hit.normal,
    };

    let transparency = material.transparency_at(alpha);
    let view_dir = normalize(&-ray_direction);

    // Light reaching this point directly from each light
    let mut diffuse_light = scene.ambient;
    let mut specular = Color::black();

    for light in scene.lights {
        // Emissive block lights only reach nearby points
        let to_light = light.position - hit.point;
        let distance = to_light.norm();
        let attenuation = light.attenuation(distance);
        if attenuation <= 0.0 {
            continue;
        }

        // Diffuse light: the more head-on the light hits the surface, the brighter
        let light_dir = to_light * (1.0 / distance);
        if dot(&hit.normal, &light_dir) <= 0.0 {
            // The light is on the other side of the face
            continue;
        }
        let facing = dot(&normal, &light_dir).max(0.0);

        // Shadow: if something blocks the light it adds nothing (or only part through glass)
        let visibility = light_visibility(&hit.point, &hit.normal, light, scene.objects);
        if visibility <= 0.0 {
            continue;
        }
        let light_color = light.color * (light.intensity * attenuation * visibility);

        diffuse_light += light_color * facing;

        // Specular light: the highlight of light reflected toward the camera
        let reflected_light = reflect(&-light_dir, &normal);
        let shine = dot(&view_dir, &reflected_light).max(0.0).powf(material.specular);
        specular += light_color * shine;
    }

    // In glass and water, diffuse light only comes from the painted part
    let coverage = if material.transparency > 0.0 { alpha.unwrap_or(1.0) } else { 1.0 };
    let mut color =
        base * diffuse_light * (material.albedo[0] * coverage) + specular * material.albedo[1];

    // Own light of emissive blocks
    if material.emission > 0.0 {
        color += base * material.emission;
    }

    // Reflection: cast another ray in the bounce direction and blend in what it finds
    if material.reflectivity > 0.0 {
        let direction = normalize(&reflect(ray_direction, &normal));
        let origin = offset_origin(&hit.point, &direction, &hit.normal);
        let reflected = cast_ray(&origin, &direction, scene, depth + 1);

        color += reflected * material.reflectivity;
    }

    // Transparency: cast another ray that bends through the surface
    if transparency > 0.0 {
        let direction = match refract(ray_direction, &normal, material.refractive_index) {
            Some(refracted) => normalize(&refracted),
            None => normalize(&reflect(ray_direction, &normal)),
        };
        let origin = offset_origin(&hit.point, &direction, &hit.normal);
        let refracted = cast_ray(&origin, &direction, scene, depth + 1);

        color += refracted * transparency;
    }

    color
}

/// Draws the scene as seen from the camera
pub fn render<O: RayIntersect>(
    framebuffer: &mut Framebuffer,
    scene: &Scene<O>,
    camera: &Camera,
    pixel_size: usize,
) {
    let pixel_size = pixel_size.max(1);
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let pixels_wide = framebuffer.width;

    // What is the same for every pixel is computed once per frame
    let eye = camera.eye();
    let basis = camera.basis();

    // Each thread takes the next free band of rows
    let bands = framebuffer.buffer_mut().chunks_mut(pixels_wide * pixel_size).enumerate();
    let bands = Mutex::new(bands);
    let threads = thread::available_parallelism().map_or(4, |n| n.get());

    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    // The lock is only held to take the next band, not to paint it
                    let Some((band, pixels)) = bands.lock().unwrap().next() else {
                        break;
                    };
                    let rows = pixels.len() / pixels_wide;

                    for x0 in (0..pixels_wide).step_by(pixel_size) {
                        // Cast the ray through the center of the square
                        let x = x0 as f32 + pixel_size as f32 * 0.5;
                        let y = (band * pixel_size) as f32 + rows as f32 * 0.5;

                        let screen_x = ((2.0 * x) / width - 1.0) * aspect_ratio;
                        let screen_y = -(2.0 * y) / height + 1.0;

                        // Ray direction in camera space, then in world space
                        let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
                        let ray_direction = basis.to_world(&ray_direction);

                        let color = cast_ray(&eye, &ray_direction, scene, 0);
                        let hex = color.tone_map().to_hex();

                        let x1 = (x0 + pixel_size).min(pixels_wide);
                        for row in pixels.chunks_mut(pixels_wide) {
                            row[x0..x1].fill(hex);
                        }
                    }
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::Cube;
    use crate::normal_map::NormalMap;
    use crate::ray_intersect::Material;
    use crate::sphere::Sphere;
    use crate::texture::{ImageTexture, Texture};

    // Ambient light used by the tests
    const AMBIENTE: f32 = 0.15;

    // Matte material of the given color
    fn mate(color: Color) -> Material {
        Material::new(Texture::Solid(color))
    }

    // Sphere on the Z axis whose red channel identifies it: `id` is the r value
    fn esfera(z: f32, id: u8) -> Sphere {
        Sphere {
            center: Vec3::new(0.0, 0.0, z),
            radius: 1.0,
            material: mate(Color::new(id, 0, 0)),
        }
    }

    // White light at the origin: hits everything on the Z axis head-on
    fn luz_de_frente() -> Light {
        blanca(Vec3::zeros(), 1.0 - AMBIENTE)
    }

    fn blanca(position: Vec3, intensity: f32) -> Light {
        Light::new(position, Color::rgb(1.0, 1.0, 1.0), intensity)
    }

    // Red channel of the color as a byte, as it would show on screen
    fn rojo(color: Color) -> u8 {
        (color.to_hex() >> 16) as u8
    }

    fn lanzar(objects: &[Sphere]) -> Color {
        lanzar_en(objects)
    }

    // Same as `lanzar` but with any object type
    fn lanzar_en<O: RayIntersect>(objects: &[O]) -> Color {
        trazar(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0), objects, &[luz_de_frente()])
    }

    // Test sky: a gradient from dark at the bottom to bright at the top
    fn cielo() -> Skybox {
        Skybox::from_fn(8, |d| Color::rgb(0.1, 0.2, 0.5 + 0.4 * d.y))
    }

    // Casts a ray in a scene with those objects and lights and the test sky
    fn trazar<O: RayIntersect>(
        origen: &Vec3,
        direccion: &Vec3,
        objects: &[O],
        lights: &[Light],
    ) -> Color {
        let skybox = cielo();
        let scene = Scene {
            objects,
            lights,
            skybox: &skybox,
            ambient: Color::rgb(AMBIENTE, AMBIENTE, AMBIENTE),
            normal_maps: true,
        };
        cast_ray(origen, direccion, &scene, 0)
    }

    #[test]
    fn pinta_la_esfera_mas_cercana_sin_importar_el_orden() {
        let cerca = esfera(-3.0, 255);
        let lejos = esfera(-5.0, 100);

        // Near one added first, then the other way around
        assert_eq!(rojo(lanzar(&[cerca, lejos])), 255);
        assert_eq!(rojo(lanzar(&[esfera(-5.0, 100), esfera(-3.0, 255)])), 255);
    }

    #[test]
    fn con_tres_esferas_gana_la_del_medio_si_es_la_mas_cercana() {
        let objects = [esfera(-6.0, 1), esfera(-2.0, 2), esfera(-4.0, 3)];
        assert_eq!(rojo(lanzar(&objects)), 2);
    }

    #[test]
    fn ignora_esferas_detras_de_la_camara() {
        let objects = [esfera(4.0, 200), esfera(-5.0, 100)];
        assert_eq!(rojo(lanzar(&objects)), 100);
    }

    #[test]
    fn si_no_hay_impacto_devuelve_el_fondo() {
        let objects = [esfera(-5.0, 100)];
        let direccion = Vec3::new(1.0, 0.0, 0.0);
        let color = trazar(&Vec3::zeros(), &direccion, &objects, &[luz_de_frente()]);
        assert_eq!(color.to_hex(), cielo().sample(&direccion).to_hex());
    }

    #[test]
    fn la_cara_que_mira_a_la_luz_es_mas_clara_que_la_que_le_da_la_espalda() {
        let objects = [esfera(-5.0, 200)];
        let origen = Vec3::zeros();
        let direccion = Vec3::new(0.0, 0.0, -1.0);

        let luz_delante = blanca(Vec3::zeros(), 1.0 - AMBIENTE);
        let luz_detras = blanca(Vec3::new(0.0, 0.0, -10.0), 1.0 - AMBIENTE);

        let iluminada = trazar(&origen, &direccion, &objects, &[luz_delante]);
        let en_sombra = trazar(&origen, &direccion, &objects, &[luz_detras]);

        assert_eq!(rojo(iluminada), 200);
        // Without direct light only ambient light remains
        assert_eq!(rojo(en_sombra), (200.0 * AMBIENTE).round() as u8);
    }

    #[test]
    fn el_albedo_difuso_escala_el_color_del_material() {
        let mut esfera = esfera(-5.0, 200);
        esfera.material.albedo = [0.5, 0.0];

        assert_eq!(rojo(lanzar(&[esfera])), 100);
    }

    #[test]
    fn el_color_sale_de_la_textura_en_el_punto_de_impacto() {
        // Red and blue checkerboard: the head-on hit point is at u = 0.75, v = 0.5
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

        // The light is right at the camera, so the highlight lands at the center of the sphere
        let brillo = |albedo_especular: f32| {
            let objects = [mate_negro(albedo_especular)];
            let luz = blanca(Vec3::zeros(), 1.0);
            rojo(trazar(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0), &objects, &[luz]))
        };
        assert_eq!(brillo(0.0), 0);
        assert_eq!(brillo(1.0), 255);
        assert_eq!(brillo(0.5), 128);
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
        // Red sphere behind the camera: it can only be seen in the reflection
        let roja = esfera(5.0, 200);

        assert_eq!(rojo(lanzar(&[espejo, roja])), 200);
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

        // Head-on, the ray goes through the glass without bending and reaches the red sphere
        assert_eq!(rojo(lanzar(&[vidrio, roja])), 200);
    }

    #[test]
    fn un_cubo_de_vidrio_deja_ver_lo_que_tiene_detras() {
        let vidrio = Cube::new(
            Vec3::new(0.0, 0.0, -3.0),
            1.0,
            Material {
                albedo: [0.0, 0.0],
                transparency: 1.0,
                refractive_index: 1.5,
                ..mate(Color::new(255, 255, 255))
            },
        );
        let roja = Cube::new(Vec3::new(0.0, 0.0, -8.0), 1.0, mate(Color::new(200, 0, 0)));

        // Mixed scene: the two cubes and a sphere out of the ray's path
        let objects: Vec<Box<dyn RayIntersect>> = vec![
            Box::new(vidrio),
            Box::new(roja),
            Box::new(esfera(5.0, 50)),
        ];
        assert_eq!(rojo(lanzar_en(&objects)), 200);
    }

    // White floor (top face at y = 0) and a light right above the point being looked at
    fn piso() -> Cube {
        Cube::new(Vec3::new(0.0, -5.0, 0.0), 10.0, mate(Color::new(200, 200, 200)))
    }

    fn mirar_el_piso<O: RayIntersect>(objects: &[O], lights: &[Light]) -> Color {
        // From above and in front toward the origin, which lies right on the floor face
        let origen = Vec3::new(0.0, 5.0, 3.0);
        trazar(&origen, &normalize(&-origen), objects, lights)
    }

    fn con_bloque_encima(material: Material) -> Vec<Box<dyn RayIntersect>> {
        let bloque = Cube::new(Vec3::new(0.0, 2.0, 0.0), 1.0, material);
        vec![Box::new(piso()), Box::new(bloque)]
    }

    #[test]
    fn un_bloque_opaco_entre_la_luz_y_el_piso_hace_sombra() {
        let luz = [luz_encima()];
        let sin_bloque = mirar_el_piso(&[piso()], &luz);
        let con_bloque = mirar_el_piso(&con_bloque_encima(mate(Color::new(0, 0, 0))), &luz);

        assert_eq!(rojo(sin_bloque), 200);
        // Only ambient light remains in the shadow
        assert_eq!(rojo(con_bloque), (200.0 * AMBIENTE).round() as u8);
    }

    #[test]
    fn un_bloque_transparente_hace_una_sombra_mas_clara() {
        let luz = [luz_encima()];
        let vidrio = Material { transparency: 0.5, ..mate(Color::new(0, 0, 0)) };
        let piedra = mate(Color::new(0, 0, 0));

        let bajo_vidrio = rojo(mirar_el_piso(&con_bloque_encima(vidrio), &luz));
        let bajo_piedra = rojo(mirar_el_piso(&con_bloque_encima(piedra), &luz));

        assert!(bajo_vidrio > bajo_piedra && bajo_vidrio < 200, "{bajo_vidrio} vs {bajo_piedra}");
    }

    // Light above the point being looked at; together with ambient it adds up to 1
    fn luz_encima() -> Light {
        blanca(Vec3::new(0.0, 4.0, 0.0), 1.0 - AMBIENTE)
    }

    #[test]
    fn dos_luces_iluminan_mas_que_una_y_la_luz_de_color_tine() {
        let una = blanca(Vec3::new(0.0, 4.0, 0.0), 0.3);
        let otra = blanca(Vec3::new(1.0, 4.0, 1.0), 0.3);

        let con_una = mirar_el_piso(&[piso()], &[una]);
        let con_dos = mirar_el_piso(&[piso()], &[blanca(Vec3::new(0.0, 4.0, 0.0), 0.3), otra]);
        assert!(con_dos.r > con_una.r);

        let azul = Light::new(Vec3::new(0.0, 4.0, 0.0), Color::rgb(0.0, 0.0, 1.0), 0.8);
        let tenido = mirar_el_piso(&[piso()], &[azul]);
        assert!(tenido.b > tenido.r * 2.0);
    }

    #[test]
    fn el_mapa_normal_hace_que_una_cara_plana_se_ilumine_distinto_en_cada_pixel() {
        // Flat texture with a ramp normal map
        let gris = Texture::Image(ImageTexture::from_pixels(1, 1, vec![Color::new(200, 200, 200)]));
        let rampa: Vec<Color> = (0..16).map(|i| Color::new((i * i) as u8, 0, 0)).collect();
        let relieve = Texture::Image(ImageTexture::from_pixels(16, 1, rampa));

        let pared = |normal_map| {
            Cube::new(
                Vec3::new(0.0, 0.0, -5.0),
                2.0,
                Material {
                    normal_map,
                    ..mate(Color::new(0, 0, 0))
                },
            )
        };
        let lisa = pared(None);
        let con_relieve = pared(NormalMap::from_texture(&relieve, 4.0));

        // Distant diagonal light, almost parallel across the whole face
        let luz = [blanca(Vec3::new(-600.0, 0.0, 600.0), 0.8)];
        let brillo = |cubo: &Cube, x: f32| {
            let cubo = Cube { material: Material { texture: gris, ..cubo.material }, ..*cubo };
            let origen = Vec3::new(x, 0.0, 0.0);
            rojo(trazar(&origen, &Vec3::new(0.0, 0.0, -1.0), &[cubo], &luz))
        };

        // Without relief the whole face gets (almost) the same light
        let diferencia = |cubo: &Cube| brillo(cubo, -0.5).abs_diff(brillo(cubo, 0.5));
        assert!(diferencia(&lisa) <= 1);
        // With relief, the steeper part of the ramp faces farther away from the light
        assert!(diferencia(&con_relieve) >= 10, "{}", diferencia(&con_relieve));
    }

    #[test]
    fn un_bloque_emisivo_brilla_sin_luz_y_alumbra_de_cerca() {
        let lampara = Material {
            emission: 1.5,
            ..mate(Color::new(200, 150, 50))
        };
        let lampara_en = |p: Vec3| Cube::new(p, 1.0, lampara);

        // With no light in the scene the emissive block still shows (and stone doesn't)
        let a_oscuras = lanzar_en::<Cube>(&[lampara_en(Vec3::new(0.0, 0.0, -5.0))]);
        assert!(a_oscuras.r > 0.9, "{a_oscuras:?}");

        // Its light reaches the nearby floor but not the far one
        let luz = |centro: Vec3| Light::from_block(centro, Color::rgb(1.0, 0.8, 0.4), 1.2, 6.0);
        let junto = Vec3::new(0.0, 1.0, -1.0);
        let cerca = mirar_el_piso(&[piso(), lampara_en(junto)], &[luz(junto)]);
        let lejos = mirar_el_piso(&[piso()], &[luz(Vec3::new(0.0, 1.0, -9.0))]);
        let sin_luz = mirar_el_piso(&[piso()], &[]);

        assert!(cerca.r > sin_luz.r + 0.2, "{cerca:?} vs {sin_luz:?}");
        assert_eq!(rojo(lejos), rojo(sin_luz));
    }

    #[test]
    fn el_bloque_de_la_luz_no_se_hace_sombra_a_si_mismo() {
        // The shadow ray hits the block, but that block is the light
        let centro = Vec3::new(0.0, 1.0, 0.0);
        let emisivo = Material {
            emission: 1.0,
            ..mate(Color::new(255, 255, 255))
        };
        let bloque = Cube::new(centro, 1.0, emisivo);
        let luz = Light::from_block(centro, Color::rgb(1.0, 1.0, 1.0), 1.0, 8.0);

        let con_luz = mirar_el_piso(&[piso(), bloque], &[luz]);
        let sin_luz = mirar_el_piso(&[piso()], &[]);
        assert!(con_luz.r > sin_luz.r + 0.3);
    }

    #[test]
    fn la_refraccion_no_desvia_un_rayo_de_frente_y_si_uno_inclinado() {
        let normal = Vec3::new(0.0, 0.0, 1.0);

        let de_frente = refract(&Vec3::new(0.0, 0.0, -1.0), &normal, 1.5).unwrap();
        assert!((de_frente - Vec3::new(0.0, 0.0, -1.0)).norm() < 1e-5);

        // Entering a denser material bends the ray toward the normal
        let entrante = normalize(&Vec3::new(1.0, 0.0, -1.0));
        let doblado = refract(&entrante, &normal, 1.5).unwrap();
        assert!(doblado.x.abs() < entrante.x.abs());
    }

    #[test]
    fn hay_reflexion_total_interna_al_salir_con_un_angulo_muy_cerrado() {
        // Ray inside the glass (same direction as the normal), very tilted
        let normal = Vec3::new(0.0, 0.0, 1.0);
        let rasante = normalize(&Vec3::new(1.0, 0.0, 0.3));

        assert!(refract(&rasante, &normal, 1.5).is_none());
    }
}
