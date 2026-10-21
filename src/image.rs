use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let image = ::image::load_from_memory(bytes)
            .map_err(|error| format!("Could not decode image: {error}"))?
            .to_rgba8();
        Ok(Self {
            width: image.width(),
            height: image.height(),
            pixels: image.into_raw(),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref()).map_err(|error| {
            format!("Could not read image {}: {error}", path.as_ref().display())
        })?;
        Self::decode(&bytes)
    }

    pub fn solid(width: u32, height: u32, color: [u8; 4]) -> Result<Self, String> {
        let size = width
            .checked_mul(height)
            .and_then(|value| value.checked_mul(4))
            .ok_or_else(|| "Image dimensions overflowed.".to_string())?;
        let mut pixels = vec![0; size as usize];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&color);
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SvgRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub color: [u8; 4],
}

pub fn parse_svg_rects(source: &str) -> Vec<SvgRect> {
    let mut rects = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.to_ascii_lowercase().find("<rect") {
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        let tag = &rest[start..start + end + 1];
        let Some(width) = numeric_attribute(tag, "width") else {
            rest = &rest[start + end + 1..];
            continue;
        };
        let Some(height) = numeric_attribute(tag, "height") else {
            rest = &rest[start + end + 1..];
            continue;
        };
        rects.push(SvgRect {
            x: numeric_attribute(tag, "x").unwrap_or(0),
            y: numeric_attribute(tag, "y").unwrap_or(0),
            width,
            height,
            color: parse_svg_color(attribute(tag, "fill").as_deref().unwrap_or("black")),
        });
        rest = &rest[start + end + 1..];
    }
    rects
}

fn numeric_attribute(tag: &str, name: &str) -> Option<i32> {
    attribute(tag, name)?.trim_end_matches("px").parse().ok()
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let start = lower.find(&(name.to_string() + "="))? + name.len() + 1;
    let value = tag[start..].trim_start();
    let quote = value.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = value[1..].find(quote)? + 1;
        Some(value[1..end].to_string())
    } else {
        Some(
            value
                .split_whitespace()
                .next()?
                .trim_end_matches('>')
                .to_string(),
        )
    }
}

fn parse_svg_color(value: &str) -> [u8; 4] {
    match value.trim().to_ascii_lowercase().as_str() {
        "red" => [220, 40, 40, 255],
        "green" => [30, 150, 80, 255],
        "blue" => [40, 90, 210, 255],
        "white" => [255, 255, 255, 255],
        _ => [0, 0, 0, 255],
    }
}

#[cfg(test)]
mod tests {
    use super::parse_svg_rects;

    #[test]
    fn parses_svg_rectangles() {
        let rects =
            parse_svg_rects(r#"<svg><rect x="2" y="3" width="10" height="5" fill="red"/></svg>"#);
        assert_eq!(rects[0].width, 10);
        assert_eq!(rects[0].color, [220, 40, 40, 255]);
    }
}
