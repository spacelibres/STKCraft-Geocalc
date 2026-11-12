//! NAIF method-string tokenization and surface-name resolution.

pub(crate) fn lex_method(input: &[u8], limit: usize) -> Result<Vec<(usize, usize)>, &'static str> {
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b' ' {
            i += 1;
            continue;
        }
        let start = i;
        if input[i] == b'"' {
            i += 1;
            let first_end = input[i..].iter().position(|&c| c == b'"');
            let Some(first_end) = first_end else {
                return Err("SPICE(SYNTAXERROR)");
            };
            i += first_end + 1;
            let mut end = i;
            let mut even = true;
            while i < input.len() {
                if input[i] == b'"' {
                    even = !even;
                    if even {
                        end = i + 1;
                    }
                } else if even {
                    break;
                }
                i += 1;
            }
            i = end;
        } else if b"/,=:".contains(&input[i]) {
            i += 1;
        } else {
            while i < input.len() && !b" /,=:\"".contains(&input[i]) {
                i += 1;
            }
        }
        if tokens.len() == limit {
            return Err("SPICE(ARRAYTOOSMALL)");
        }
        tokens.push((start, i));
    }
    Ok(tokens)
}

/// Components of a NAIF geometry method string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSurfaceMethod {
    /// Shape keyword, `DSK` or `ELLIPSOID`.
    pub shape: String,
    /// Sub-point keyword, or an empty string when unspecified.
    pub subtype: String,
    /// Whether prioritized DSK selection is requested.
    pub prioritized: bool,
    /// Resolved surface IDs; an empty list selects all DSK surfaces.
    pub surfaces: Vec<i32>,
    /// Limb point-definition keyword, if present.
    pub point: String,
    /// Terminator-type keyword, if present.
    pub terminator: String,
}

/// A native short error produced while parsing a surface method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceMethodError(String);
impl std::fmt::Display for SurfaceMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SurfaceMethodError {}

/// Parse a SPICE geometry method and resolve its surface names for `body`.
///
/// `max_surfaces` is the receiving operation's surface-list capacity. This
/// parses shared syntax only: each geometry operation must additionally
/// validate which parsed shape and clauses it accepts.
pub fn parse_surface_method(
    resolver: &mut crate::SurfaceNameResolver,
    pool: &crate::Pck,
    body: i32,
    method: &str,
    max_surfaces: usize,
) -> Result<ParsedSurfaceMethod, SurfaceMethodError> {
    parse_method(resolver, pool, body, method, max_surfaces).map_err(SurfaceMethodError)
}

fn parse_method(
    resolver: &mut crate::SurfaceNameResolver,
    pool: &crate::Pck,
    body: i32,
    method: &str,
    max_surfaces: usize,
) -> Result<ParsedSurfaceMethod, String> {
    let syntax = || "SPICE(BADMETHODSYNTAX)".to_string();
    let mut normalized = Vec::new();
    let mut blank = false;
    for byte in method.bytes() {
        if byte == b' ' {
            if !normalized.is_empty() && !blank {
                normalized.push(byte);
            }
            blank = true;
        } else {
            normalized.push(byte.to_ascii_uppercase());
            blank = false;
        }
        if normalized.len() == 999 {
            break;
        }
    }
    let ranges = lex_method(&normalized, 100).map_err(str::to_owned)?;
    let tokens: Vec<&[u8]> = ranges.iter().map(|&(a, b)| &normalized[a..b]).collect();
    let shapes: Vec<&[u8]> = tokens
        .iter()
        .copied()
        .filter(|t| *t == b"DSK" || *t == b"ELLIPSOID")
        .collect();
    if shapes.len() != 1 {
        return Err(syntax());
    }
    let shape = std::str::from_utf8(shapes[0]).unwrap().to_string();
    let mut out = ParsedSurfaceMethod {
        shape,
        subtype: String::new(),
        prioritized: true,
        surfaces: Vec::new(),
        point: String::new(),
        terminator: String::new(),
    };
    if out.shape == "ELLIPSOID" {
        let legacy: String = method
            .chars()
            .filter(|c| *c != ' ')
            .map(|c| c.to_ascii_uppercase())
            .collect();
        if legacy == "ELLIPSOID" {
            return Ok(out);
        }
        for (suffix, subtype) in [("NEARPOINT", "NEAR POINT"), ("INTERCEPT", "INTERCEPT")] {
            if [
                format!("{suffix}:ELLIPSOID"),
                format!("ELLIPSOID:{suffix}"),
                format!("{suffix}/ELLIPSOID"),
                format!("ELLIPSOID/{suffix}"),
            ]
            .contains(&legacy)
            {
                out.subtype = subtype.into();
                return Ok(out);
            }
        }
    }
    let mut t = vec![b"/".as_slice()];
    t.extend(tokens);
    let mut i = 0;
    let mut surface_clause = false;
    let mut subtype_seen = false;
    while i < t.len() {
        if out.shape == "DSK" && i + 1 == t.len() {
            break;
        }
        if out.shape == "DSK" && t[i] != b"/" {
            i += 1;
            continue;
        }
        if t[i] != b"/" || i + 1 >= t.len() {
            return Err(syntax());
        }
        i += 1;
        match t[i] {
            b"DSK" if out.shape == "DSK" => {}
            b"ELLIPSOID" if out.shape == "ELLIPSOID" => {}
            b"UNPRIORITIZED" if out.shape == "DSK" => {
                if !out.prioritized {
                    return Err(syntax());
                }
                out.prioritized = false;
            }
            b"INTERCEPT" | b"NADIR" => {
                if subtype_seen {
                    return Err(syntax());
                }
                subtype_seen = true;
                out.subtype = std::str::from_utf8(t[i]).unwrap().into();
            }
            b"NEAR" if out.shape == "ELLIPSOID" => {
                if subtype_seen {
                    return Err(syntax());
                }
                subtype_seen = true;
                if t.get(i + 1) == Some(&b"POINT".as_slice()) {
                    out.subtype = "NADIR".into();
                }
            }
            b"TANGENT" | b"GUIDED" => {
                if !out.point.is_empty() {
                    return Err(syntax());
                }
                out.point = std::str::from_utf8(t[i]).unwrap().into();
            }
            b"UMBRAL" | b"PENUMBRAL" => {
                if !out.terminator.is_empty() {
                    return Err(syntax());
                }
                out.terminator = std::str::from_utf8(t[i]).unwrap().into();
            }
            b"SURFACES" if out.shape == "DSK" => {
                if surface_clause || i + 2 >= t.len() || t[i + 1] != b"=" {
                    return Err(syntax());
                }
                surface_clause = true;
                i += 2;
                loop {
                    let token = t[i];
                    let name = if token.starts_with(b"\"") {
                        if token.len() <= 2 {
                            return Err(syntax());
                        }
                        &token[1..token.len() - 1]
                    } else {
                        token
                    };
                    let value = resolver
                        .resolve_bytes(pool, name, body)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| "SPICE(IDCODENOTFOUND)".to_string())?;
                    if out.surfaces.len() == max_surfaces {
                        return Err("SPICE(TOOMANYSURFACES)".into());
                    }
                    out.surfaces.push(value);
                    if i + 1 == t.len() || t[i + 1] == b"/" {
                        break;
                    }
                    if t[i + 1] != b"," || i + 2 >= t.len() {
                        return Err(syntax());
                    }
                    i += 2;
                }
            }
            _ => return Err(syntax()),
        }
        i += 1;
    }
    if out.shape == "DSK" && out.prioritized {
        return Err("SPICE(BADPRIORITYSPEC)".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pck, SurfaceNameResolver};

    #[test]
    fn native_method_parser_capture() {
        let pool = Pck::from_text("\\begindata\nNAIF_SURFACE_NAME=('Alpha Surface' 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA')\nNAIF_SURFACE_CODE=(100 7)\nNAIF_SURFACE_BODY=(301 301)\n\\begintext\n").unwrap();
        let mut resolver = SurfaceNameResolver::new();
        let mut count = 0;
        for row in include_str!("../../docs/review-2026-11-12/surface-names/method-reference.txt")
            .lines()
            .skip(1)
        {
            let (maximum, row) = row.split_once('|').unwrap();
            let (method, expected) = row.split_once('|').unwrap();
            let actual =
                match parse_method(&mut resolver, &pool, 301, method, maximum.parse().unwrap()) {
                    Ok(output) => format!(
                        "N|{}|{}|{}|{}|{}|{}",
                        output.shape,
                        output.subtype,
                        i32::from(output.prioritized),
                        output
                            .surfaces
                            .iter()
                            .map(i32::to_string)
                            .collect::<Vec<_>>()
                            .join(","),
                        output.point,
                        output.terminator
                    ),
                    Err(error) => format!("E|{error}"),
                };
            assert_eq!(actual, expected, "method {method:?}");
            count += 1;
        }
        assert_eq!(count, 11338);
    }
}
