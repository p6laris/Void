use anyhow::{bail, Context, Result};
use ratatui::style::Color;

pub fn parse_color(raw: &str) -> Result<Color> {
    let s = raw.trim();
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex);
    }
    bail!("invalid color `{raw}`: expected #rrggbb or #rgb");
}

pub fn resolve_color(
    raw: &str,
    palette: &std::collections::HashMap<String, String>,
) -> Result<Color> {
    let s = raw.trim();
    if s.starts_with('#') {
        return parse_color(s);
    }
    palette
        .get(s)
        .map(|v| parse_color(v))
        .transpose()
        .and_then(|opt| opt.context(format!("unknown palette key `{s}`")))
}

fn parse_hex(hex: &str) -> Result<Color> {
    match hex.len() {
        3 => {
            let r = hex_digit(hex.as_bytes()[0])?;
            let g = hex_digit(hex.as_bytes()[1])?;
            let b = hex_digit(hex.as_bytes()[2])?;
            Ok(Color::Rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = from_hex_pair(&hex[0..2])?;
            let g = from_hex_pair(&hex[2..4])?;
            let b = from_hex_pair(&hex[4..6])?;
            Ok(Color::Rgb(r, g, b))
        }
        _ => bail!("invalid hex color `#{hex}`"),
    }
}

fn hex_digit(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => bail!("invalid hex digit `{b}`"),
    }
}

fn from_hex_pair(pair: &str) -> Result<u8> {
    let bytes = pair.as_bytes();
    Ok(hex_digit(bytes[0])? * 16 + hex_digit(bytes[1])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_digit_hex() {
        assert_eq!(parse_color("#1e1e2e").unwrap(), Color::Rgb(30, 30, 46));
    }

    #[test]
    fn resolves_palette_key() {
        let mut palette = std::collections::HashMap::new();
        palette.insert("blue".into(), "#89b6fa".into());
        assert_eq!(
            resolve_color("blue", &palette).unwrap(),
            Color::Rgb(137, 182, 250)
        );
    }
}
