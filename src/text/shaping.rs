use super::{glyph::GlyphBitmap, rasterizer::Rasterizer};

#[derive(Debug)]
pub struct PositionedGlyph {
    pub bitmap: GlyphBitmap,
    pub x: i32,
}

pub fn shape_text(rasterizer: &Rasterizer, text: &str, size: f32) -> Vec<PositionedGlyph> {
    let mut cursor = 0;
    text.chars()
        .map(|character| {
            let bitmap = rasterizer.glyph(character, size);
            let x = cursor + bitmap.metrics.xmin;
            cursor += bitmap.metrics.advance_width.round() as i32;
            PositionedGlyph { bitmap, x }
        })
        .collect()
}
