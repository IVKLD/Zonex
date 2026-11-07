mod engine_resolver;
mod extract;
mod navigation;
mod open;
mod ops;

pub use engine_resolver::{get_engine_for_path, EngineType};

use crate::archive::{ArchiveEngine, ArchiveEntry};
use crate::fuse::FuseMountManager;
use anyhow::Result;
use navigation::NavigationState;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ArchiveManager {
    current_path: Option<PathBuf>,
    all_entries: Vec<ArchiveEntry>,
    nav: NavigationState,
    active_engine_type: Option<EngineType>,
    fuse_mount: Option<FuseMountManager>,
}

impl ArchiveManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_engine_for_path(path: &Path) -> Result<Box<dyn ArchiveEngine>> {
        engine_resolver::get_engine_for_path(path)
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    pub fn current_folder(&self) -> &Path {
        &self.nav.current_folder
    }

    pub fn fuse_mount_path(&self) -> Option<&Path> {
        self.fuse_mount.as_ref().map(|m| m.mount_path())
    }

    pub fn enter_folder(&mut self, folder: PathBuf) {
        self.nav.enter_folder(folder);
    }

    pub fn navigate_back(&mut self) -> bool {
        self.nav.navigate_back()
    }

    pub fn navigate_forward(&mut self) -> bool {
        self.nav.navigate_forward()
    }

    pub fn visible_entries(&self) -> Vec<ArchiveEntry> {
        self.nav.visible_entries(&self.all_entries)
    }

    pub fn open_archive(&mut self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        self.open_archive_with_password(path, None)
    }

    pub fn open_archive_with_password(
        &mut self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Vec<ArchiveEntry>> {
        let (entries, engine_type, fuse_mount) = open::open_archive_with_password(path, password)?;
        self.current_path = Some(path.to_path_buf());
        self.all_entries = entries;
        self.nav.reset();
        self.active_engine_type = Some(engine_type);
        self.fuse_mount = fuse_mount;
        Ok(self.visible_entries())
    }
}
