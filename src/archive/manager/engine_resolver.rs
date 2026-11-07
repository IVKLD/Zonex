use crate::archive::detector::{detect, DetectedFormat};
use crate::archive::libarchive_engine::LibarchiveEngine;
use crate::archive::tar_engine::{TarCompression, TarEngine};
use crate::archive::zip_engine::ZipEngine;
use crate::archive::ArchiveEngine;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Zip,
    TarGz,
    TarZst,
    Tar,
    Libarchive,
}

impl EngineType {
    pub fn create_engine(self) -> Box<dyn ArchiveEngine> {
        match self {
            Self::Zip => Box::new(ZipEngine),
            Self::TarGz => Box::new(TarEngine::new(TarCompression::Gzip)),
            Self::TarZst => Box::new(TarEngine::new(TarCompression::Zstd)),
            Self::Tar => Box::new(TarEngine::new(TarCompression::None)),
            Self::Libarchive => Box::new(LibarchiveEngine),
        }
    }
}

impl From<DetectedFormat> for EngineType {
    fn from(format: DetectedFormat) -> Self {
        match format {
            DetectedFormat::Zip => Self::Zip,
            DetectedFormat::TarGz => Self::TarGz,
            DetectedFormat::TarZst => Self::TarZst,
            DetectedFormat::Tar => Self::Tar,
        }
    }
}

pub fn candidate_engine_types(path: &Path) -> Vec<EngineType> {
    let mut types = Vec::with_capacity(5);
    if let Some(format) = detect(path) {
        types.push(EngineType::from(format));
    }
    for fallback in [
        EngineType::Libarchive,
        EngineType::Zip,
        EngineType::TarGz,
        EngineType::TarZst,
        EngineType::Tar,
    ] {
        if !types.contains(&fallback) {
            types.push(fallback);
        }
    }
    types
}

pub fn get_engine_for_path(path: &Path) -> Result<Box<dyn ArchiveEngine>> {
    let engine_type = detect(path).map_or(EngineType::Libarchive, EngineType::from);
    Ok(engine_type.create_engine())
}
