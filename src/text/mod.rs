mod font;
mod glyph;
mod rasterizer;
mod shaping;

pub use font::{FontFace, FontManager};
pub use rasterizer::Rasterizer;
pub use shaping::shape_text;

use std::sync::OnceLock;

static RASTERIZER: OnceLock<Result<Rasterizer, String>> = OnceLock::new();

pub fn embedded_rasterizer() -> Result<&'static Rasterizer, String> {
    match RASTERIZER.get_or_init(Rasterizer::embedded) {
        Ok(rasterizer) => Ok(rasterizer),
        Err(error) => Err(error.clone()),
    }
}

pub fn measure_text(text: &str, size: f32) -> i32 {
    let Ok(rasterizer) = embedded_rasterizer() else {
        return 0;
    };
    shape_text(rasterizer, text, size)
        .last()
        .map(|glyph| glyph.x + glyph.bitmap.metrics.width as i32)
        .unwrap_or(0)
}

pub fn draw_text(
    frame: &mut [u8],
    width: i32,
    height: i32,
    x: i32,
    baseline: i32,
    text: &str,
    size: f32,
    color: [u8; 4],
) -> Result<(), String> {
    let rasterizer = embedded_rasterizer()?;
    let glyphs = shape_text(rasterizer, text, size);

    for glyph in glyphs {
        let top = baseline - glyph.bitmap.metrics.height as i32 - glyph.bitmap.metrics.ymin;
        for row in 0..glyph.bitmap.metrics.height {
            for column in 0..glyph.bitmap.metrics.width {
                let coverage = glyph.bitmap.coverage[row * glyph.bitmap.metrics.width + column];
                if coverage == 0 {
                    continue;
                }

                let px = x + glyph.x + column as i32;
                let py = top + row as i32;
                if px < 0 || py < 0 || px >= width || py >= height {
                    continue;
                }

                let index = ((py * width + px) * 4) as usize;
                if index + 3 >= frame.len() {
                    continue;
                }
                let alpha = u16::from(coverage);
                for channel in 0..3 {
                    let background = u16::from(frame[index + channel]);
                    let foreground = u16::from(color[channel]);
                    frame[index + channel] =
                        ((foreground * alpha + background * (255 - alpha)) / 255) as u8;
                }
                frame[index + 3] = 255;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{embedded_rasterizer, measure_text, FontManager};

    #[test]
    fn embedded_font_loads_and_measures_text() {
        assert!(embedded_rasterizer().is_ok());
        assert!(measure_text("Rust", 16.0) > 0);
    }

    #[test]
    fn font_manager_loads_bundled_font_bytes() {
        let bytes = include_bytes!("../../assets/fonts/SIXTY.TTF");
        let mut manager = FontManager::default();
        manager.load_bytes("Demo", bytes).unwrap();
        assert!(manager.get("demo").is_some());
    }
}
