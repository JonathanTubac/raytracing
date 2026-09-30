//! Image reading and writing without external libraries

use std::fs;

use crate::inflate::zlib_decompress;

/// RGBA image in memory, row by row from top to bottom
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<[u8; 4]>,
}

/// Reads a PNG or PPM image, depending on the file extension
pub fn load_image(path: &str) -> Result<Image, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if path.ends_with(".png") {
        decode_png(&bytes)
    } else {
        parse_ppm(&bytes)
    }
}

/// Binary PPM (P6) with 255 as the maximum value per channel
fn parse_ppm(bytes: &[u8]) -> Result<Image, String> {
    // Header: "P6", width, height and maximum; anything after a '#' is a comment
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

    // A single whitespace separates the header from the pixels
    let data = bytes.get(pos + 1..).unwrap_or(&[]);
    if data.len() < width * height * 3 {
        return Err("el PPM tiene menos pixeles de los que dice".into());
    }

    let pixels = data
        .chunks_exact(3)
        .take(width * height)
        .map(|p| [p[0], p[1], p[2], 255])
        .collect();

    Ok(Image { width, height, pixels })
}

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Non-interlaced PNG, any color type, 1, 2, 4, 8 or 16 bits per channel
fn decode_png(bytes: &[u8]) -> Result<Image, String> {
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("no es un PNG".into());
    }

    let mut header = None;
    let mut palette: &[u8] = &[];
    let mut transparency: &[u8] = &[];
    let mut compressed = Vec::new();

    let mut pos = PNG_SIGNATURE.len();
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data = bytes.get(pos + 8..pos + 8 + len).ok_or("bloque de PNG cortado")?;
        match kind {
            b"IHDR" => header = Some(PngHeader::parse(data)?),
            b"PLTE" => palette = data,
            b"tRNS" => transparency = data,
            b"IDAT" => compressed.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        // Length, type, data and CRC
        pos += 12 + len;
    }

    let header = header.ok_or("el PNG no tiene encabezado")?;
    let raw = zlib_decompress(&compressed)?;
    let samples = unfilter(&raw, &header)?;
    header.to_rgba(&samples, palette, transparency)
}

struct PngHeader {
    width: usize,
    height: usize,
    bit_depth: usize,
    /// 0 = gray, 2 = RGB, 3 = palette, 4 = gray + alpha, 6 = RGBA
    color_type: u8,
}

impl PngHeader {
    fn parse(data: &[u8]) -> Result<PngHeader, String> {
        if data.len() < 13 {
            return Err("encabezado de PNG cortado".into());
        }
        let header = PngHeader {
            width: u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize,
            height: u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize,
            bit_depth: data[8] as usize,
            color_type: data[9],
        };
        if data[12] != 0 {
            return Err("no se soportan PNG entrelazados".into());
        }
        if ![1, 2, 4, 8, 16].contains(&header.bit_depth) || header.channels() == 0 {
            return Err("formato de PNG invalido".into());
        }
        Ok(header)
    }

    /// How many values each pixel has
    fn channels(&self) -> usize {
        match self.color_type {
            0 | 3 => 1,
            2 => 3,
            4 => 2,
            6 => 4,
            _ => 0,
        }
    }

    /// Bytes per row, not counting the filter byte
    fn stride(&self) -> usize {
        (self.width * self.channels() * self.bit_depth).div_ceil(8)
    }

    /// Byte distance to the same value of the previous pixel (at least 1), used by filters
    fn bytes_per_pixel(&self) -> usize {
        (self.channels() * self.bit_depth).div_ceil(8)
    }

    /// Reads value number `index` from a row, whatever its size
    fn sample(&self, row: &[u8], index: usize) -> u16 {
        match self.bit_depth {
            8 => row[index] as u16,
            16 => u16::from_be_bytes([row[index * 2], row[index * 2 + 1]]),
            bits => {
                // Values under 8 bits are packed together, the first one in the high bits
                let bit = index * bits;
                let shift = 8 - bits - bit % 8;
                ((row[bit / 8] >> shift) as u16) & ((1 << bits) - 1)
            }
        }
    }

    /// Scales a `bit_depth` bit value to 0..=255
    fn to_byte(&self, value: u16) -> u8 {
        match self.bit_depth {
            16 => (value >> 8) as u8,
            bits => (value as u32 * 255 / ((1 << bits) - 1)) as u8,
        }
    }

    fn to_rgba(
        &self,
        samples: &[u8],
        palette: &[u8],
        transparency: &[u8],
    ) -> Result<Image, String> {
        // With tRNS in gray or RGB images, a single color (in its original value) is transparent
        let transparent_value = |i: usize| -> Option<u16> {
            transparency.get(i * 2..i * 2 + 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
        };
        let channels = self.channels();

        let mut pixels = Vec::with_capacity(self.width * self.height);
        for row in samples.chunks_exact(self.stride()) {
            for x in 0..self.width {
                let value = |channel: usize| self.sample(row, x * channels + channel);
                let byte = |channel: usize| self.to_byte(value(channel));

                let pixel = match self.color_type {
                    0 => {
                        let alpha = if transparent_value(0) == Some(value(0)) { 0 } else { 255 };
                        [byte(0), byte(0), byte(0), alpha]
                    }
                    2 => {
                        let is_key = (0..3).all(|ch| transparent_value(ch) == Some(value(ch)));
                        [byte(0), byte(1), byte(2), if is_key { 0 } else { 255 }]
                    }
                    3 => {
                        let index = value(0) as usize;
                        let rgb = palette
                            .get(index * 3..index * 3 + 3)
                            .ok_or("indice de paleta fuera de rango")?;
                        let alpha = transparency.get(index).copied().unwrap_or(255);
                        [rgb[0], rgb[1], rgb[2], alpha]
                    }
                    4 => [byte(0), byte(0), byte(0), byte(1)],
                    _ => [byte(0), byte(1), byte(2), byte(3)],
                };
                pixels.push(pixel);
            }
        }

        Ok(Image { width: self.width, height: self.height, pixels })
    }
}

/// Undoes each row's filter
fn unfilter(raw: &[u8], header: &PngHeader) -> Result<Vec<u8>, String> {
    let stride = header.stride();
    let bpp = header.bytes_per_pixel();
    if raw.len() < header.height * (stride + 1) {
        return Err("el PNG tiene menos datos de los que dice su tamano".into());
    }

    let mut out = vec![0u8; header.height * stride];
    for y in 0..header.height {
        let filter = raw[y * (stride + 1)];
        let line = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, current) = out.split_at_mut(y * stride);
        let current = &mut current[..stride];
        let above = if y > 0 { &done[(y - 1) * stride..] } else { &[][..] };

        for i in 0..stride {
            let a = if i >= bpp { current[i - bpp] } else { 0 };
            let b = above.get(i).copied().unwrap_or(0);
            let c = if i >= bpp { above.get(i - bpp).copied().unwrap_or(0) } else { 0 };

            let prediction = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => paeth(a, b, c),
                _ => return Err(format!("filtro de PNG desconocido: {filter}")),
            };
            current[i] = line[i].wrapping_add(prediction);
        }
    }
    Ok(out)
}

/// Picks the neighbor (left, up or up-left) closest to a + b - c
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let (pa, pb, pc) = ((p - a as i16).abs(), (p - b as i16).abs(), (p - c as i16).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Saves 0xRRGGBB pixels (the framebuffer format) as a PNG
pub fn save_png(path: &str, width: usize, height: usize, pixels: &[u32]) -> std::io::Result<()> {
    fs::write(path, encode_png(width, height, pixels))
}

fn encode_png(width: usize, height: usize, pixels: &[u32]) -> Vec<u8> {
    // Each row starts with the filter type (0 = none) followed by the RGB bytes
    let mut raw = Vec::with_capacity(height * (1 + width * 3));
    for row in pixels.chunks_exact(width) {
        raw.push(0);
        for &color in row {
            raw.extend_from_slice(&[(color >> 16) as u8, (color >> 8) as u8, color as u8]);
        }
    }

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();

    // IHDR: size, 8 bits per channel, color type 2 (RGB), no interlacing
    let mut header = Vec::new();
    header.extend_from_slice(&(width as u32).to_be_bytes());
    header.extend_from_slice(&(height as u32).to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    write_chunk(&mut png, b"IHDR", &header);

    write_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    write_chunk(&mut png, b"IEND", &[]);
    png
}

/// A PNG chunk: length, type, data and the CRC of the type plus the data
fn write_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend_from_slice(kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

/// Uncompressed zlib format: stored blocks and the Adler-32 checksum
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
        assert_eq!(imagen.pixels, vec![[255, 0, 0, 255], [0, 0, 255, 255]]);
    }

    #[test]
    fn rechaza_un_ppm_incompleto_o_de_otro_tipo() {
        assert!(parse_ppm(b"P6\n2 2\n255\n\x00\x00\x00").is_err());
        assert!(parse_ppm(b"P3\n1 1\n255\n0 0 0").is_err());
    }

    #[test]
    fn crc_y_adler_dan_los_valores_conocidos() {
        // zlib reference values for the string "123456789"
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

    #[test]
    fn lo_que_se_guarda_en_png_se_lee_igual() {
        let original = [0xFF0000, 0x00FF00, 0x0000FF, 0x123456, 0xFFFFFF, 0x000000];
        let imagen = decode_png(&encode_png(3, 2, &original)).unwrap();

        assert_eq!((imagen.width, imagen.height), (3, 2));
        let leido: Vec<u32> = imagen
            .pixels
            .iter()
            .map(|p| (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32)
            .collect();
        assert_eq!(leido, original);
    }

    #[test]
    fn el_filtro_paeth_elige_el_vecino_mas_parecido() {
        assert_eq!(paeth(10, 20, 10), 20);
        assert_eq!(paeth(20, 10, 10), 20);
        assert_eq!(paeth(10, 10, 30), 10);
    }

    #[test]
    fn las_texturas_de_minecraft_se_leen_igual_que_con_pil() {
        // Checksums computed with Python's PIL on the same files
        let esperado = [
            ("bricks", 150165, 61716026),
            ("cobblestone", 163249, 66882931),
            ("diamond_block", 209516, 82364626),
            ("dirt", 141452, 59335235),
            ("glass", 129490, 39526584),
            ("glowstone", 164416, 64269563),
            ("gold_block", 197469, 68857484),
            ("grass_block_side", 141957, 59306065),
            ("grass_block_top", 178509, 71187102),
            ("iron_block", 234274, 85180302),
            ("lava_still", 2946668, 364034723),
            ("mossy_cobblestone", 148108, 62587658),
            ("oak_leaves", 118271, 47432280),
            ("oak_log", 127976, 56269579),
            ("oak_log_top", 153881, 62126321),
            ("oak_planks", 160423, 63086451),
            ("obsidian", 78158, 45456701),
            ("sand", 216186, 78972055),
            ("sea_lantern", 1046069, 981060995),
            ("stone", 161700, 66822126),
            ("stone_bricks", 159163, 65265800),
            ("water_still", 5826636, 849043403),
            // From the sky: the sun (palette) and the clouds (1 bit gray with a transparent color)
            ("sun", 325228, 727741800),
            ("clouds", 18465060, 887664418),
        ];

        for (nombre, suma, ponderada) in esperado {
            let ruta = format!("{}/assets/textures/{nombre}.png", env!("CARGO_MANIFEST_DIR"));
            let imagen = load_image(&ruta).unwrap_or_else(|e| panic!("{nombre}: {e}"));

            let bytes = imagen.pixels.iter().flatten().map(|&b| b as u64);
            let pesos = imagen.pixels.iter().enumerate().map(|(i, p)| {
                let valor = p[0] as u64 + 2 * p[1] as u64 + 3 * p[2] as u64 + 5 * p[3] as u64;
                (i as u64 + 1) * valor
            });
            assert_eq!(bytes.sum::<u64>(), suma, "{nombre}");
            assert_eq!(pesos.sum::<u64>() % 1_000_000_007, ponderada, "{nombre}");
        }
    }
}
