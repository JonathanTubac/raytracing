//! Lectura y escritura de imagenes sin librerias externas.
//!
//! - Las texturas se leen en PPM binario (P6): un encabezado de texto y despues los bytes
//!   RGB de cada pixel, sin compresion.
//! - Las capturas se guardan en PNG, que se ve en GitHub y en cualquier visor. Se escribe
//!   sin comprimir (bloques "stored" de deflate), que es valido y mucho mas simple.

use std::fs;

use crate::color::Color;

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color>,
}

/// Lee un archivo PPM binario (P6) con 255 como valor maximo por canal
pub fn load_ppm(path: &str) -> Result<Image, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    parse_ppm(&bytes)
}

fn parse_ppm(bytes: &[u8]) -> Result<Image, String> {
    // El encabezado son 4 palabras separadas por espacios ("P6", ancho, alto, maximo);
    // lo que va de un '#' al final de la linea es un comentario
    let mut pos = 0;
    let mut next_word = || -> Result<String, String> {
        loop {
            match bytes.get(pos) {
                Some(b'#') => {
                    while bytes.get(pos).is_some_and(|&b| b != b'\n') {
                        pos += 1;
                    }
                }
                Some(b) if b.is_ascii_whitespace() => pos += 1,
                Some(_) => break,
                None => return Err("el encabezado del PPM esta incompleto".into()),
            }
        }
        let start = pos;
        while bytes.get(pos).is_some_and(|b| !b.is_ascii_whitespace()) {
            pos += 1;
        }
        Ok(String::from_utf8_lossy(&bytes[start..pos]).into_owned())
    };

    if next_word()? != "P6" {
        return Err("solo se soportan PPM binarios (P6)".into());
    }
    let mut number = || -> Result<usize, String> {
        next_word()?.parse().map_err(|_| "numero invalido en el encabezado".to_string())
    };
    let (width, height, max) = (number()?, number()?, number()?);
    if max != 255 {
        return Err(format!("solo se soporta 255 como valor maximo, no {max}"));
    }

    // Un unico espacio separa el encabezado de los pixeles
    let data = bytes.get(pos + 1..).unwrap_or(&[]);
    if data.len() < width * height * 3 {
        return Err("el PPM tiene menos pixeles de los que dice".into());
    }

    let pixels = data
        .chunks_exact(3)
        .take(width * height)
        .map(|p| Color::new(p[0], p[1], p[2]))
        .collect();

    Ok(Image { width, height, pixels })
}

/// Guarda pixeles en formato 0xRRGGBB (el del framebuffer) como PNG
pub fn save_png(path: &str, width: usize, height: usize, pixels: &[u32]) -> std::io::Result<()> {
    fs::write(path, encode_png(width, height, pixels))
}

fn encode_png(width: usize, height: usize, pixels: &[u32]) -> Vec<u8> {
    // Cada fila empieza con el tipo de filtro (0 = ninguno) y sigue con los bytes RGB
    let mut raw = Vec::with_capacity(height * (1 + width * 3));
    for row in pixels.chunks_exact(width) {
        raw.push(0);
        for &color in row {
            raw.extend_from_slice(&[(color >> 16) as u8, (color >> 8) as u8, color as u8]);
        }
    }

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();

    // IHDR: tamano, 8 bits por canal, tipo de color 2 (RGB), sin entrelazado
    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    write_chunk(&mut png, b"IHDR", &header);

    write_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    write_chunk(&mut png, b"IEND", &[]);
    png
}

/// Un bloque de PNG: largo, tipo, datos y el CRC del tipo mas los datos
fn write_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

/// Envuelve los datos en formato zlib sin comprimir: bloques de hasta 65535 bytes
/// guardados tal cual, y al final la suma de verificacion Adler-32
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = data.chunks(65535).collect();

    for (i, block) in blocks.iter().enumerate() {
        let is_last = i + 1 == blocks.len();
        out.push(is_last as u8);
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    if blocks.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }

    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_un_ppm_con_comentarios() {
        let mut archivo = b"P6\n# hecho a mano\n2 1\n255\n".to_vec();
        archivo.extend_from_slice(&[255, 0, 0, 0, 0, 255]);

        let imagen = parse_ppm(&archivo).unwrap();
        assert_eq!((imagen.width, imagen.height), (2, 1));
        assert_eq!(imagen.pixels[0].to_hex(), 0xFF0000);
        assert_eq!(imagen.pixels[1].to_hex(), 0x0000FF);
    }

    #[test]
    fn rechaza_un_ppm_incompleto_o_de_otro_tipo() {
        assert!(parse_ppm(b"P6\n2 2\n255\n\x00\x00\x00").is_err());
        assert!(parse_ppm(b"P3\n1 1\n255\n0 0 0").is_err());
    }

    #[test]
    fn crc_y_adler_dan_los_valores_conocidos() {
        // Valores de referencia de zlib para la cadena "123456789"
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(adler32(b"123456789"), 0x091E_01DE);
    }

    #[test]
    fn el_png_tiene_firma_encabezado_y_final_correctos() {
        let png = encode_png(2, 2, &[0xFF0000, 0x00FF00, 0x0000FF, 0xFFFFFF]);

        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 2);
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
    }
}
