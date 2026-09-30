//! Deflate decompressor (RFC 1951), the compression format PNG uses

/// Reads bits one at a time, starting with the least significant bit of each byte
struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bit: u32,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, pos: 0, bit: 0 }
    }

    fn bit(&mut self) -> Result<u32, String> {
        let byte = *self.data.get(self.pos).ok_or("los datos comprimidos se cortaron")?;
        let value = (byte >> self.bit) & 1;
        self.bit += 1;
        if self.bit == 8 {
            self.bit = 0;
            self.pos += 1;
        }
        Ok(value as u32)
    }

    /// Reads `count` bits as a number; the first bit read is the least significant
    fn bits(&mut self, count: u32) -> Result<u32, String> {
        let mut value = 0;
        for i in 0..count {
            value |= self.bit()? << i;
        }
        Ok(value)
    }

    /// Skips the rest of the current byte (stored blocks start on a byte boundary)
    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.pos += 1;
        }
    }
}

/// Canonical Huffman code: the code length of each symbol is enough
struct Huffman {
    /// How many symbols there are of each length (0 to 15 bits)
    counts: [u16; 16],
    /// Symbols sorted by code length and, for equal lengths, by value
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Huffman {
        let mut counts = [0u16; 16];
        for &len in lengths {
            counts[len as usize] += 1;
        }
        counts[0] = 0;

        // Where each length starts inside `symbols`
        let mut offsets = [0u16; 16];
        for len in 1..15 {
            offsets[len + 1] = offsets[len] + counts[len];
        }

        let mut symbols = vec![0; lengths.len()];
        for (symbol, &len) in lengths.iter().enumerate() {
            if len != 0 {
                symbols[offsets[len as usize] as usize] = symbol as u16;
                offsets[len as usize] += 1;
            }
        }

        Huffman { counts, symbols }
    }

    /// Reads bits until a code is complete
    fn decode(&self, reader: &mut BitReader) -> Result<u16, String> {
        let mut code: i32 = 0;
        let mut first: i32 = 0;
        let mut index: i32 = 0;

        for len in 1..16 {
            code |= reader.bit()? as i32;
            let count = self.counts[len] as i32;
            if code - first < count {
                return Ok(self.symbols[(index + code - first) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("codigo de Huffman invalido".into())
    }
}

// Base value and extra bits of the length and distance symbols
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
    131, 163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

// Order of the code lengths of the code that describes the other two codes
const CODE_LENGTH_ORDER: [usize; 19] =
    [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

/// Decompresses zlib: 2 header bytes, deflate data and the final checksum (not checked)
pub fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 2 || data[0] & 0x0F != 8 {
        return Err("no es un flujo zlib con deflate".into());
    }
    inflate(&data[2..])
}

pub fn inflate(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = BitReader::new(data);
    let mut out = Vec::new();

    loop {
        let is_last = reader.bit()? == 1;
        match reader.bits(2)? {
            0 => stored_block(&mut reader, &mut out)?,
            1 => {
                let (literals, distances) = fixed_codes();
                huffman_block(&mut reader, &mut out, &literals, &distances)?;
            }
            2 => {
                let (literals, distances) = dynamic_codes(&mut reader)?;
                huffman_block(&mut reader, &mut out, &literals, &distances)?;
            }
            _ => return Err("tipo de bloque deflate invalido".into()),
        }
        if is_last {
            return Ok(out);
        }
    }
}

/// Stored block: length, its complement, and the raw bytes
fn stored_block(reader: &mut BitReader, out: &mut Vec<u8>) -> Result<(), String> {
    reader.align();
    let header = reader
        .data
        .get(reader.pos..reader.pos + 4)
        .ok_or("encabezado de bloque sin comprimir cortado")?;
    let len = u16::from_le_bytes([header[0], header[1]]) as usize;
    let nlen = u16::from_le_bytes([header[2], header[3]]) as usize;
    if len != !nlen & 0xFFFF {
        return Err("largo de bloque sin comprimir inconsistente".into());
    }

    let start = reader.pos + 4;
    let bytes = reader.data.get(start..start + len).ok_or("bloque sin comprimir cortado")?;
    out.extend_from_slice(bytes);
    reader.pos = start + len;
    Ok(())
}

/// Fixed codes defined by the standard, for blocks that don't carry their own
fn fixed_codes() -> (Huffman, Huffman) {
    let mut lengths = [0u8; 288];
    lengths[..144].fill(8);
    lengths[144..256].fill(9);
    lengths[256..280].fill(7);
    lengths[280..].fill(8);
    (Huffman::new(&lengths), Huffman::new(&[5; 30]))
}

/// Codes carried inside the block
fn dynamic_codes(reader: &mut BitReader) -> Result<(Huffman, Huffman), String> {
    let literal_count = reader.bits(5)? as usize + 257;
    let distance_count = reader.bits(5)? as usize + 1;
    let code_length_count = reader.bits(4)? as usize + 4;

    let mut code_lengths = [0u8; 19];
    for &position in &CODE_LENGTH_ORDER[..code_length_count] {
        code_lengths[position] = reader.bits(3)? as u8;
    }
    let code_length_code = Huffman::new(&code_lengths);

    let mut lengths = Vec::with_capacity(literal_count + distance_count);
    while lengths.len() < literal_count + distance_count {
        let symbol = code_length_code.decode(reader)?;
        match symbol {
            0..=15 => lengths.push(symbol as u8),
            // 16: repeat the previous length 3 to 6 times
            16 => {
                let previous = *lengths.last().ok_or("repeticion sin largo anterior")?;
                let times = 3 + reader.bits(2)?;
                lengths.extend(std::iter::repeat_n(previous, times as usize));
            }
            // 17 and 18: repeat a zero length, 3 to 10 times or 11 to 138 times
            17 => {
                let times = 3 + reader.bits(3)?;
                lengths.extend(std::iter::repeat_n(0, times as usize));
            }
            _ => {
                let times = 11 + reader.bits(7)?;
                lengths.extend(std::iter::repeat_n(0, times as usize));
            }
        }
    }
    if lengths.len() > literal_count + distance_count {
        return Err("demasiados largos de codigo".into());
    }

    let literals = Huffman::new(&lengths[..literal_count]);
    let distances = Huffman::new(&lengths[literal_count..]);
    Ok((literals, distances))
}

fn huffman_block(
    reader: &mut BitReader,
    out: &mut Vec<u8>,
    literals: &Huffman,
    distances: &Huffman,
) -> Result<(), String> {
    loop {
        let symbol = literals.decode(reader)? as usize;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Ok(()),
            _ => {
                let i = symbol - 257;
                if i >= LENGTH_BASE.len() {
                    return Err("simbolo de largo invalido".into());
                }
                let extra = reader.bits(LENGTH_EXTRA[i] as u32)? as usize;
                let length = LENGTH_BASE[i] as usize + extra;

                let d = distances.decode(reader)? as usize;
                if d >= DIST_BASE.len() {
                    return Err("simbolo de distancia invalido".into());
                }
                let distance = DIST_BASE[d] as usize + reader.bits(DIST_EXTRA[d] as u32)? as usize;
                if distance > out.len() {
                    return Err("la distancia apunta antes del inicio".into());
                }

                // Byte by byte, because the copy can overlap what it is writing
                let start = out.len() - distance;
                for k in 0..length {
                    out.push(out[start + k]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_un_bloque_sin_comprimir() {
        // Final block (1), type 0; length 3 and its complement; "abc"
        let datos = [0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c'];
        assert_eq!(inflate(&datos).unwrap(), b"abc");
    }

    #[test]
    fn lee_un_bloque_con_codigos_fijos_y_copias() {
        // "hola hola hola" compressed by zlib (level 9): uses fixed codes and a copy
        let datos = [
            0x78, 0xDA, 0xCB, 0xC8, 0xCF, 0x49, 0x54, 0xC8, 0x80, 0x11, 0x00, 0x26, 0xFC, 0x05,
            0x2D,
        ];
        assert_eq!(zlib_decompress(&datos).unwrap(), b"hola hola hola");
    }

    #[test]
    fn lee_un_bloque_con_codigos_dinamicos() {
        // Uneven frequencies: zlib builds its own Huffman codes
        let datos = [
            0x78, 0xDA, 0x2D, 0x90, 0x81, 0x0D, 0x44, 0x31, 0x08, 0x42, 0x57, 0x61,
            0x35, 0x50, 0xF7, 0x5F, 0xE1, 0x1E, 0xFD, 0xD7, 0xA4, 0xA9, 0x82, 0x82,
            0xD6, 0x72, 0xBC, 0xB3, 0x63, 0xAF, 0x67, 0x3C, 0x71, 0x11, 0xFB, 0x4C,
            0x52, 0x6C, 0xAF, 0x39, 0xF8, 0x9C, 0x89, 0x42, 0xB1, 0x13, 0xED, 0x8E,
            0xBE, 0x96, 0x1C, 0xDC, 0xD2, 0x3F, 0x59, 0x90, 0x48, 0x4F, 0x0C, 0x09,
            0x34, 0x36, 0xCA, 0xA4, 0x81, 0xB7, 0x92, 0xE4, 0x57, 0x3F, 0x40, 0xF8,
            0x4C, 0xCB, 0xC4, 0xAD, 0x25, 0x74, 0xEA, 0x95, 0x26, 0xA9, 0x0B, 0x8A,
            0x08, 0xF1, 0x3C, 0x06, 0xB7, 0x57, 0xC4, 0xDD, 0xA9, 0x4A, 0xFD, 0xF5,
            0x38, 0xFF, 0xFB, 0x48, 0x3B, 0xBA, 0xAC, 0x47, 0x16, 0xD5, 0x37, 0x67,
            0x95, 0x6A, 0x4A, 0x19, 0x0B, 0x96, 0x99, 0xCB, 0x31, 0xD3, 0x77, 0x58,
            0x32, 0x1B, 0x4E, 0xBF, 0xC3, 0xEA, 0xCE, 0xDD, 0x9F, 0x07, 0x25, 0x55,
            0x31, 0xF9, 0x01, 0x06, 0xCD, 0x6D, 0xEC
        ];
        let texto = zlib_decompress(&datos).unwrap();

        assert_eq!(texto.len(), 300);
        assert!(texto.starts_with(b"a abadcdcaadaccacba abaaaeacacadacdeabaa"));
        assert_eq!(texto.iter().filter(|&&b| b == b'a').count(), 120);
        assert_eq!(texto.iter().map(|&b| b as u32).sum::<u32>(), 28139);
    }

    #[test]
    fn rechaza_datos_cortados() {
        assert!(inflate(&[0x05]).is_err());
    }
}
