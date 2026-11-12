//! Stateful NAIF surface-name lookup for a text-kernel pool.
use crate::frame::parse_f64;
use crate::Pck;
use std::fmt;

/// A native short error from surface-name translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceNameError(&'static str);
impl fmt::Display for SurfaceNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for SurfaceNameError {}

/// A stateful resolver. Keep one instance for each independent kernel-pool history.
#[derive(Debug, Default)]
pub struct SurfaceNameResolver {
    revision: Option<u64>,
    initialized: bool,
    active: bool,
    entries: Vec<(Vec<u8>, i32, i32)>,
    slots: Vec<(usize, usize)>,
}

fn normalized(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut pending = false;
    for &byte in input {
        if byte == b' ' {
            if !out.is_empty() {
                pending = true;
            }
        } else {
            if pending {
                out.push(b' ');
                pending = false;
            }
            out.push(byte.to_ascii_uppercase());
        }
    }
    out.truncate(36);
    while out.last() == Some(&b' ') {
        out.pop();
    }
    out
}

// ZZHASH2 with the 2003-bucket surface-name table, after removing spaces.
fn name_bucket(name: &[u8]) -> usize {
    const VALUES: [u8; 129] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 41, 66, 43, 44, 45, 47, 65, 49, 50, 48, 51, 62, 37, 39, 40, 1, 2, 3, 4, 5, 6, 7,
        8, 9, 10, 59, 60, 61, 52, 63, 64, 42, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 53, 58, 55, 46, 38, 67, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 54, 57, 56, 68, 0, 0,
    ];
    let mut hash = 0usize;
    for &byte in name.iter().filter(|&&byte| byte != b' ') {
        hash = (VALUES[(byte as usize).min(128)] as usize + hash * 68) % 2003;
    }
    (hash * 68) % 2003
}

impl SurfaceNameResolver {
    /// Create an initially unpopulated resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve already-tokenized DSK method surface names in order.
    ///
    /// An empty list selects all surfaces. Unknown names are rejected with
    /// the short error used by NAIF's DSK method parser. This method does not
    /// parse method-string syntax or enforce a caller-specific list limit.
    pub fn resolve_for_method(
        &mut self,
        pool: &Pck,
        names: &[&str],
        body: i32,
    ) -> Result<Vec<i32>, SurfaceNameError> {
        names
            .iter()
            .map(|name| {
                self.resolve(pool, name, body)?
                    .ok_or(SurfaceNameError("SPICE(IDCODENOTFOUND)"))
            })
            .collect()
    }

    /// Translate a name or numeric string for a body, preserving pool update history.
    pub fn resolve(
        &mut self,
        pool: &Pck,
        name: &str,
        body: i32,
    ) -> Result<Option<i32>, SurfaceNameError> {
        self.resolve_bytes(pool, name.as_bytes(), body)
    }

    pub(crate) fn resolve_bytes(
        &mut self,
        pool: &Pck,
        name: &[u8],
        body: i32,
    ) -> Result<Option<i32>, SurfaceNameError> {
        if name.is_empty() {
            return Err(SurfaceNameError("SPICE(EMPTYSTRING)"));
        }
        let (variables, revision) = pool.surface_pool();
        if self.revision != Some(revision) {
            self.revision = Some(revision);
            let result = (|| -> Result<(), SurfaceNameError> {
                self.active = false;
                let names = variables.get("NAIF_SURFACE_NAME");
                let codes = variables.get("NAIF_SURFACE_CODE");
                let bodies = variables.get("NAIF_SURFACE_BODY");
                if names.is_some() != codes.is_some() || names.is_some() != bodies.is_some() {
                    return Err(SurfaceNameError("SPICE(BADSURFACEMAP)"));
                }
                if let (Some(names), Some(codes), Some(bodies)) = (names, codes, bodies) {
                    self.active = true;
                    let names = names
                        .as_strings()
                        .ok_or(SurfaceNameError("SPICE(BADVARIABLETYPE)"))?;
                    let codes = codes
                        .as_list()
                        .ok_or(SurfaceNameError("SPICE(BADVARIABLETYPE)"))?;
                    let bodies = bodies
                        .as_list()
                        .ok_or(SurfaceNameError("SPICE(BADVARIABLETYPE)"))?;
                    if names.len() > 2000 || codes.len() > 2000 || bodies.len() > 2000 {
                        return Err(SurfaceNameError("SPICE(TOOMANYSURFACES)"));
                    }
                    if names.len() != codes.len() || names.len() != bodies.len() {
                        return Err(SurfaceNameError("SPICE(ARRAYSIZEMISMATCH)"));
                    }
                    let integer = |text: &str| -> Result<i32, SurfaceNameError> {
                        let value = parse_f64(text)
                            .map_err(|_| SurfaceNameError("SPICE(INTOUTOFRANGE)"))?;
                        if value < i32::MIN as f64 || value > i32::MAX as f64 {
                            return Err(SurfaceNameError("SPICE(INTOUTOFRANGE)"));
                        }
                        Ok(value.round() as i32)
                    };
                    let mut parsed_codes = Vec::with_capacity(codes.len());
                    for (index, value) in codes.iter().enumerate() {
                        let code = integer(value)?;
                        if let Some(entry) = self.entries.get_mut(index) {
                            entry.2 = code;
                        }
                        parsed_codes.push(code);
                    }
                    let mut parsed_bodies = Vec::with_capacity(bodies.len());
                    for (index, value) in bodies.iter().enumerate() {
                        let body = integer(value)?;
                        if let Some(entry) = self.entries.get_mut(index) {
                            entry.1 = body;
                        }
                        parsed_bodies.push(body);
                    }
                    let mut entries = Vec::with_capacity(names.len());
                    for (index, ((name, code), body)) in names
                        .iter()
                        .zip(parsed_codes)
                        .zip(parsed_bodies)
                        .enumerate()
                    {
                        let bytes = name.as_bytes();
                        let key = normalized(&bytes[..bytes.len().min(36)]);
                        if key.is_empty() {
                            return Err(SurfaceNameError("SPICE(BLANKNAMEASSIGNED)"));
                        }
                        if let Some(entry) = self.entries.get_mut(index) {
                            entry.0 = key.clone();
                        }
                        entries.push((key, body, code));
                    }
                    let mut slots: Vec<(usize, usize)> = Vec::new();
                    for index in (0..entries.len()).rev() {
                        let (name, body, _) = &entries[index];
                        let bucket = name_bucket(name);
                        if !slots.iter().any(|&(hash, other)| {
                            hash == bucket && entries[other].0 == *name && entries[other].1 == *body
                        }) {
                            slots.push((bucket, index));
                        }
                    }
                    self.entries = entries;
                    self.slots = slots;
                }
                Ok(())
            })();
            if let Err(error) = result {
                if !self.initialized {
                    self.revision = None;
                }
                return Err(error);
            }
            self.initialized = true;
        }
        if self.active {
            let key = normalized(name);
            let bucket = name_bucket(&key);
            if let Some((_, index)) = self.slots.iter().find(|&&(hash, index)| {
                let (stored, stored_body, _) = &self.entries[index];
                hash == bucket && *stored_body == body && *stored == key
            }) {
                return Ok(Some(self.entries[*index].2));
            }
        }
        // NPARSI handles numeric tokens, not text-kernel @date literals.
        let Ok(name) = std::str::from_utf8(name) else {
            return Ok(None);
        };
        if name.starts_with('@') {
            return Ok(None);
        }
        let number = match parse_surface_number(name) {
            Ok(number) => number.trunc(),
            Err(_) => return Ok(None),
        };
        if number < i32::MIN as f64 || number > i32::MAX as f64 {
            return Ok(None);
        }
        Ok(Some(number as i32))
    }
}

// NPARSD stops accumulating an underflowing exponent but keeps scanning for
// invalid characters. Its overflow check occurs before each exponent digit.
fn parse_surface_number(name: &str) -> Result<f64, ()> {
    let text: String = name
        .chars()
        .filter(|c| *c != ' ' && *c != ',')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if text.starts_with('@') {
        return Err(());
    }
    let unsigned = text.trim_start_matches(['+', '-']);
    let Some(position) = unsigned.find(['E', 'D']) else {
        return parse_f64(&text).map_err(|_| ());
    };
    let mantissa = &unsigned[..position];
    let mut integer = 0.0;
    let mut fraction = 0.0;
    let mut decimal = false;
    let mut zero_integer = true;
    let mut shift: i64 = 0;
    for c in mantissa.chars() {
        if c == '.' && !decimal {
            decimal = true;
            zero_integer = integer == 0.0;
        } else {
            let digit = c.to_digit(10).ok_or(())? as f64;
            if !decimal {
                if integer < 1e15 {
                    integer = integer * 10.0 + digit;
                } else {
                    shift += 1;
                }
            } else if zero_integer && fraction < 1e15 {
                fraction = fraction * 10.0 + digit;
                shift -= 1;
            }
        }
    }
    let exponent = &unsigned[position + 1..];
    let (sign, digits) = if let Some(rest) = exponent.strip_prefix('-') {
        (-1_i64, rest)
    } else {
        (1_i64, exponent.strip_prefix('+').unwrap_or(exponent))
    };
    let mut value = 0_i64;
    for c in digits.chars() {
        let digit = i64::from(c.to_digit(10).ok_or(())?);
        if value + shift > 308 {
            return Err(());
        }
        if value + shift >= -309 {
            value = value * 10 + sign * digit;
        }
    }
    let prefix_len = text.len() - unsigned.len();
    if prefix_len > 1 {
        return Err(());
    }
    let prefix = &text[..prefix_len];
    let mantissa = if mantissa == "." { "" } else { mantissa };
    let canonical = format!("{prefix}{mantissa}E{value}");
    parse_f64(&canonical).map_err(|_| ())
}

#[cfg(test)]
mod native_hash_tests {
    use super::name_bucket;
    #[test]
    fn native_method_lexer_token_ranges() {
        let mut count = 0;
        for row in
            include_str!("../../docs/review-2026-11-12/surface-names/method-lexer-reference.txt")
                .lines()
                .skip(1)
        {
            let (input, expected) = row.split_once('|').unwrap();
            let input = input.replace("\\t", "\t");
            let actual = match crate::shapes::dsk_method::lex_method(input.as_bytes(), 100) {
                Ok(tokens) => format!(
                    "N|{}",
                    tokens
                        .iter()
                        .map(|(a, b)| format!("{}:{}", a + 1, b))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                Err(error) => format!("E|{error}"),
            };
            assert_eq!(actual, expected, "input {input:?}");
            count += 1;
        }
        assert_eq!(count, 2016);
    }
    #[test]
    fn native_surface_hash_two_thousand_names() {
        let rows: Vec<_> =
            include_str!("../../docs/review-2026-11-12/surface-names/hash-buckets.csv")
                .lines()
                .skip(1)
                .collect();
        assert_eq!(rows.len(), 2000);
        for (index, row) in rows.iter().enumerate() {
            let (name, expected) = row.split_once(',').unwrap();
            assert_eq!(name, format!("NAME{index:04}"));
            assert_eq!(
                name_bucket(name.as_bytes()),
                expected.parse::<usize>().unwrap(),
                "{name}"
            );
        }
    }
    #[test]
    fn native_surface_hash_buckets() {
        for (name, expected) in [
            ("A", 748),
            ("B", 816),
            ("C", 884),
            ("ALPHA SURFACE", 166),
            ("0", 68),
            ("9", 680),
            ("A-B", 1220),
            ("A_B", 1838),
            ("A/B", 1071),
            ("@", 853),
            ("\\", 1941),
            ("?", 346),
            ("", 0),
            ("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ", 1852),
        ] {
            assert_eq!(name_bucket(name.as_bytes()), expected, "{name}");
        }
    }
}
