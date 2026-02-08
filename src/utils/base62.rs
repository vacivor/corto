use std::sync::OnceLock;

const ALPHABET: &[u8; 62] = b"hNzYdDs6xriR7elMCZIAq1BtwHjQXSOup2FEn8cJfo4Gyk0W3bPULva5K9VgmT";

fn lookup_table() -> &'static [i8; 256] {
    static TABLE: OnceLock<[i8; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [-1i8; 256];
        for (idx, &ch) in ALPHABET.iter().enumerate() {
            table[ch as usize] = idx as i8;
        }
        table
    })
}

pub fn encode(num: i64) -> String {
    if num <= 0 {
        return "0".to_string();
    }
    let mut buf = Vec::new();
    let mut n = num as u64;
    while n > 0 {
        let rem = (n % 62) as usize;
        buf.push(ALPHABET[rem]);
        n /= 62;
    }
    buf.reverse();
    String::from_utf8(buf).expect("base62 encoding to be valid utf8")
}

pub fn decode(input: &str) -> Result<i64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("base62 input is empty".to_string());
    }

    let table = lookup_table();
    let mut value: i64 = 0;
    for ch in trimmed.bytes() {
        let idx = table[ch as usize];
        if idx < 0 {
            return Err(format!("invalid base62 character: {}", ch as char));
        }
        let digit = idx as i64;

        value = value
            .checked_mul(62)
            .and_then(|v| v.checked_add(digit))
            .ok_or_else(|| "base62 value overflow".to_string())?;
    }

    Ok(value)
}
