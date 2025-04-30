pub enum Endianness {
    Little,
    Big,
}

pub fn hex_decode(hex_string: &str, endianness: &Endianness) -> Option<Vec<u8>> {
    if hex_string.len() % 2 != 0 {
        return None;
    }

    let bytes: Option<Vec<u8>> = (0..hex_string.len())
        .step_by(2)
        .map(|i| {
            let byte = u8::from_str_radix(&hex_string[i..i + 2], 16).ok()?;
            match endianness {
                Endianness::Little => Some(byte),
                Endianness::Big => Some(byte.reverse_bits()),
            }
        })
        .collect();

    bytes
}
