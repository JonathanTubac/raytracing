use std::sync::Mutex;
use std::thread;

use crate::camera::Camera;
use crate::color::Color;
use crate::framebuffer::Framebuffer;
use crate::light::Light;
use crate::math::{dot, normalize, Vec3};
use crate::ray_intersect::{Intersect, RayIntersect};

// Color del cielo abajo (horizonte hacia el suelo) y arriba. Los rayos que no golpean nada,
// incluidos los que salen de reflejos y refracciones, toman su color de este degradado.
const FONDO_ABAJO: Color = Color::new(25, 25, 40);
const FONDO_ARRIBA: Color = Color::new(110, 140, 200);

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

/// Cuanta luz de `light` llega a `point`: 1 si no hay nada en el camino y 0 si un objeto
/// opaco la tapa. Los objetos transparentes dejan pasar una parte, asi el vidrio y el agua
/// dan sombras mas claras que la piedra.
fn light_visibility<O: RayIntersect>(
    point: &Vec3,
    normal: &Vec3,
    light: &Light,
    objects: &[O],
) -> f32 {
    let to_light = light.position - point;
    let distance = to_light.norm();
    let direction = to_light * (1.0 / distance);
    let origin = offset_origin(point, &direction, normal);

    // A diferencia de un rayo normal no importa cual objeto esta mas cerca: cualquiera que
    // este entre el punto y la luz hace sombra
    let mut visibility = 1.0;
    for object in objects {
        let hit = object.ray_intersect(&origin, &direction);
        if !hit.is_intersecting || hit.distance >= distance {
            continue;
        }

        // Cuanta luz deja pasar depende del pixel de la textura donde cruza el rayo: por
        // el hueco entre las hojas pasa toda, por el marco del vidrio nada
        let (_, alpha) = hit.material.texture.sample(hit.u, hit.v, hit.face);
        if !hit.material.is_hole(alpha) {
            visibility *= hit.material.transparency_at(alpha);
        }
        if visibility <= 0.0 {
            return 0.0;
        }
    }
    visibility
}

pub fn cast_ray<O: RayIntersect>(
    ray_origin: &Vec3,
    ray_direction: &Vec3,
    objects: &[O],
    lights: &[Light],
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

    let Some(mut hit) = closest else {
        return fondo(ray_direction);
    };
    let material = hit.material;

    // El color de la superficie viene de la textura del material en el punto de impacto,
    // con la imagen que corresponde a esa cara del bloque
    let (base, alpha) = material.texture.sample(hit.u, hit.v, hit.face);

    // Hueco en la textura (entre las hojas): el rayo sigue de largo desde ahi. No cuenta
    // como un rebote, porque en realidad no choco con nada.
    if material.is_hole(alpha) {
        let origin = offset_origin(&hit.point, ray_direction, &hit.normal);
        return cast_ray(&origin, ray_direction, objects, lights, depth);
    }

    // Si se ve la cara desde adentro de un bloque opaco (la cara de atras de un bloque de
    // hojas, vista a traves de un hueco) se ilumina como si mirara hacia la camara
    if material.transparency == 0.0 && dot(&hit.normal, ray_direction) > 0.0 {
        hit.normal = -hit.normal;
    }

    let transparency = material.transparency_at(alpha);
    let view_dir = normalize(&-ray_direction);

    // Luz que llega directo de cada luz a este punto. Empieza con la luz ambiente: la
    // minima que recibe cualquier superficie aunque ninguna luz le llegue de frente.
    let mut diffuse_light = Color::rgb(AMBIENTE, AMBIENTE, AMBIENTE);
    let mut specular = Color::black();

    for light in lights {
        // Luz difusa: entre mas de frente le llega la luz a la superficie, mas brillante.
        // Es el coseno del angulo entre la normal y la direccion hacia la luz.
        let light_dir = normalize(&(light.position - hit.point));
        let facing = dot(&hit.normal, &light_dir);
        if facing <= 0.0 {
            // La luz esta del otro lado de la superficie: no aporta nada, ni hace falta
            // revisar si hay algo tapandola
            continue;
        }

        // Sombra: si algo tapa la luz, esta no aporta nada (o solo una parte si es vidrio)
        let visibility = light_visibility(&hit.point, &hit.normal, light, objects);
        if visibility <= 0.0 {
            continue;
        }
        let light_color = light.color * (light.intensity * visibility);

        diffuse_light += light_color * facing;

        // Luz especular: el brillo de la luz sobre la superficie, que se ve solo cuando la
        // camara esta cerca de la direccion en la que rebota la luz
        let reflected_light = reflect(&-light_dir, &hit.normal);
        let shine = dot(&view_dir, &reflected_light).max(0.0).powf(material.specular);
        specular += light_color * shine;
    }

    // En los materiales transparentes con textura (vidrio, agua) la luz difusa sale solo
    // de la parte pintada: el marco del vidrio se ve con su color y el centro no aporta
    let coverage = if material.transparency > 0.0 { alpha.unwrap_or(1.0) } else { 1.0 };
    let mut color =
        base * diffuse_light * (material.albedo[0] * coverage) + specular * material.albedo[1];

    // Reflejo: se lanza otro rayo en la direccion de rebote y se mezcla lo que encuentre
    if material.reflectivity > 0.0 {
        let direction = normalize(&reflect(ray_direction, &hit.normal));
        let origin = offset_origin(&hit.point, &direction, &hit.normal);
        let reflected = cast_ray(&origin, &direction, objects, lights, depth + 1);

        color += reflected * material.reflectivity;
    }

    // Transparencia: se lanza otro rayo que atraviesa la superficie doblandose. Si no puede
    // atravesarla (reflexion total interna), el rayo rebota por dentro.
    if transparency > 0.0 {
        let direction = match refract(ray_direction, &hit.normal, material.refractive_index) {
            Some(refracted) => normalize(&refracted),
            None => normalize(&reflect(ray_direction, &hit.normal)),
        };
        let origin = offset_origin(&hit.point, &direction, &hit.normal);
        let refracted = cast_ray(&origin, &direction, objects, lights, depth + 1);

        color += refracted * transparency;
    }

    color
}

pub fn render<O: RayIntersect>(
    framebuffer: &mut Framebuffer,
    objects: &[O],
    camera: &Camera,
    lights: &[Light],
) {
    let width = framebuffer.width as f32;
    let height = framebuffer.height as f32;
    let aspect_ratio = width / height;
    let eye = camera.eye();
    let pixels_wide = framebuffer.width;

    // Cada pixel es independiente de los demas, asi que se reparten entre todos los nucleos
    // del procesador. El trabajo se reparte por filas: cada hilo toma la siguiente fila
    // libre cuando termina la suya. Asi, si un hilo cae en filas caras (vidrio, espejos)
    // los demas siguen avanzando en vez de quedarse esperando a que termine.
    let rows = Mutex::new(framebuffer.buffer_mut().chunks_mut(pixels_wide).enumerate());
    let threads = thread::available_parallelism().map_or(4, |n| n.get());

    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    // El candado solo se toma para sacar la siguiente fila, no para pintarla
                    let Some((y, row)) = rows.lock().unwrap().next() else {
                        break;
                    };

                    for (x, pixel) in row.iter_mut().enumerate() {
                        // Mapea el pixel a espacio de pantalla [-1, 1]
                        let screen_x = (2.0 * x as f32) / width - 1.0;
                        let screen_y = -(2.0 * y as f32) / height + 1.0;

                        // Ajuste por aspect ratio
                        let screen_x = screen_x * aspect_ratio;

                        // Direccion del rayo en el espacio de la camara, y luego en el del mundo
                        let ray_direction = normalize(&Vec3::new(screen_x, screen_y, -1.0));
                        let ray_direction = camera.basis_change(&ray_direction);

                        // Se lanza el rayo desde la camara. El color que devuelve puede
                        // pasar de 1.0, asi que se comprime al rango de la pantalla.
                        let color = cast_ray(&eye, &ray_direction, objects, lights, 0);
                        *pixel = color.tone_map().to_hex();
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
    use crate::ray_intersect::Material;
    use crate::sphere::Sphere;
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

    // Luz blanca en el origen: le pega de frente a todo lo que esta sobre el eje Z. Junto
    // con la luz ambiente suma exactamente 1, asi los colores de estas pruebas salen tal
    // cual, sin oscurecer ni aclarar.
    fn luz_de_frente() -> Light {
        blanca(Vec3::zeros(), 1.0 - AMBIENTE)
    }

    fn blanca(position: Vec3, intensity: f32) -> Light {
        Light::new(position, Color::rgb(1.0, 1.0, 1.0), intensity)
    }

    // Canal rojo del color como byte, igual que se veria en pantalla
    fn rojo(color: Color) -> u8 {
        (color.to_hex() >> 16) as u8
    }

    fn lanzar(objects: &[Sphere]) -> Color {
        lanzar_en(objects)
    }

    // Igual que `lanzar` pero con cualquier tipo de objeto
    fn lanzar_en<O: RayIntersect>(objects: &[O]) -> Color {
        cast_ray(
            &Vec3::zeros(),
            &Vec3::new(0.0, 0.0, -1.0),
            objects,
            &[luz_de_frente()],
            0,
        )
    }

    #[test]
    fn pinta_la_esfera_mas_cercana_sin_importar_el_orden() {
        let cerca = esfera(-3.0, 255);
        let lejos = esfera(-5.0, 100);

        // Cerca agregada primero, y despues al reves
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
        let color = cast_ray(&Vec3::zeros(), &direccion, &objects, &[luz_de_frente()], 0);
        assert_eq!(color.to_hex(), fondo(&direccion).to_hex());
    }

    #[test]
    fn la_cara_que_mira_a_la_luz_es_mas_clara_que_la_que_le_da_la_espalda() {
        let objects = [esfera(-5.0, 200)];
        let origen = Vec3::zeros();
        let direccion = Vec3::new(0.0, 0.0, -1.0);

        let luz_delante = blanca(Vec3::zeros(), 1.0 - AMBIENTE);
        let luz_detras = blanca(Vec3::new(0.0, 0.0, -10.0), 1.0 - AMBIENTE);

        let iluminada = cast_ray(&origen, &direccion, &objects, &[luz_delante], 0);
        let en_sombra = cast_ray(&origen, &direccion, &objects, &[luz_detras], 0);

        assert_eq!(rojo(iluminada), 200);
        // Sin luz directa solo queda la luz ambiente
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
        let brillo = |albedo_especular: f32| {
            let objects = [mate_negro(albedo_especular)];
            let luz = blanca(Vec3::zeros(), 1.0);
            rojo(cast_ray(&Vec3::zeros(), &Vec3::new(0.0, 0.0, -1.0), &objects, &[luz], 0))
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
        // Esfera roja detras de la camara: solo se puede ver a traves del reflejo. La cara
        // que se ve mira hacia la camara, que es donde esta la luz.
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

        // De frente el rayo atraviesa el vidrio sin desviarse y llega a la esfera roja
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

        // Escena mezclada: los dos cubos y una esfera fuera del camino del rayo
        let objects: Vec<Box<dyn RayIntersect>> = vec![
            Box::new(vidrio),
            Box::new(roja),
            Box::new(esfera(5.0, 50)),
        ];
        assert_eq!(rojo(lanzar_en(&objects)), 200);
    }

    // Piso blanco (cara de arriba en y = 0) y una luz justo encima del punto que se mira
    fn piso() -> Cube {
        Cube::new(Vec3::new(0.0, -5.0, 0.0), 10.0, mate(Color::new(200, 200, 200)))
    }

    fn mirar_el_piso<O: RayIntersect>(objects: &[O], lights: &[Light]) -> Color {
        // Desde arriba y adelante hacia el origen, que queda justo sobre la cara del piso
        let origen = Vec3::new(0.0, 5.0, 3.0);
        cast_ray(&origen, &normalize(&-origen), objects, lights, 0)
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
        // En la sombra solo queda la luz ambiente
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

    // Luz encima del punto del piso que se mira, con la intensidad justa para que junto con
    // el ambiente sume 1
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
