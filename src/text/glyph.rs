use fontdue::Metrics;

#[derive(Debug)]
pub struct GlyphBitmap {
    pub metrics: Metrics,
    pub coverage: Vec<u8>,
}
