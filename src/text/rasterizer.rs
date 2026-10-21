use super::{font::FontFace, glyph::GlyphBitmap};

#[derive(Debug)]
pub struct Rasterizer {
    face: FontFace,
}

impl Rasterizer {
    pub fn embedded() -> Result<Self, String> {
        Ok(Self {
            face: FontFace::embedded()?,
        })
    }

    pub fn glyph(&self, character: char, size: f32) -> GlyphBitmap {
        let (metrics, coverage) = self.face.font.rasterize(character, size.max(1.0));
        GlyphBitmap { metrics, coverage }
    }
}
