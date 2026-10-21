use fontdue::{Font as DueFont, FontSettings};
use std::{collections::HashMap, path::Path};

const FONT_DATA: &[u8] = include_bytes!("../../assets/fonts/SIXTY.TTF");

#[derive(Debug)]
pub struct FontFace {
    pub(crate) font: DueFont,
}

impl FontFace {
    pub fn embedded() -> Result<Self, String> {
        let font = DueFont::from_bytes(FONT_DATA, FontSettings::default())
            .map_err(|error| format!("failed to load embedded SIXTY font: {error}"))?;
        Ok(Self { font })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let font = DueFont::from_bytes(bytes, FontSettings::default())
            .map_err(|error| format!("failed to load web font: {error}"))?;
        Ok(Self { font })
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|error| format!("could not read font {}: {error}", path.as_ref().display()))?;
        Self::from_bytes(&bytes)
    }
}

#[derive(Debug, Default)]
pub struct FontManager {
    faces: HashMap<String, FontFace>,
}

impl FontManager {
    pub fn load_bytes(&mut self, family: impl Into<String>, bytes: &[u8]) -> Result<(), String> {
        self.faces.insert(
            family.into().to_ascii_lowercase(),
            FontFace::from_bytes(bytes)?,
        );
        Ok(())
    }

    pub fn load_file(
        &mut self,
        family: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<(), String> {
        self.faces.insert(
            family.into().to_ascii_lowercase(),
            FontFace::from_file(path)?,
        );
        Ok(())
    }

    pub fn get(&self, family: &str) -> Option<&FontFace> {
        self.faces.get(&family.to_ascii_lowercase())
    }
}
