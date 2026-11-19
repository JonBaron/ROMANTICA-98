pub struct Pcx {
    pub width: usize,
    /// 256 farger, 8 bit per komponent.
    pub palette: [u8; 768],
}

/// Pakker ut en 8-bit PCX rett inn i `dest` som en lineær pikselstrøm, slik originalen gjorde.
/// Bytes-per-linje i headeren ignoreres, så bilder med padding får en stride på
/// bredde+1 (gjelder font26.pcx).
pub fn decode(buf: &[u8], dest: &mut [u8]) -> Pcx {
    assert!(buf.len() >= 128 + 769, "for liten PCX-fil");
    let width = u16::from_le_bytes([buf[8], buf[9]]) as usize + 1;
    let height = u16::from_le_bytes([buf[10], buf[11]]) as usize + 1;
    let total = width * height;

    let mut src = 128;
    let mut n = 0;
    while n <= total && src < buf.len() {
        let b = buf[src];
        src += 1;
        if b & 0xC0 == 0xC0 {
            let count = (b & 0x3F) as usize;
            let value = buf.get(src).copied().unwrap_or(0);
            src += 1;
            for i in n..(n + count).min(dest.len()) {
                dest[i] = value;
            }
            n += count;
        } else {
            if n < dest.len() {
                dest[n] = b;
            }
            n += 1;
        }
    }

    let mut p = buf.len() - 0x301;
    while p < buf.len() && buf[p] != 0x0C {
        p += 1;
    }
    let mut palette = [0u8; 768];
    let start = (p + 1).min(buf.len());
    let avail = (buf.len() - start).min(768);
    palette[..avail].copy_from_slice(&buf[start..start + avail]);

    Pcx { width, palette }
}
