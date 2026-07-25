use super::detector::{DetectedFormat, FormatDetector};
use super::libarchive_engine::LibarchiveEngine;
use super::tar_engine::{TarCompression, TarEngine};
use super::zip_engine::ZipEngine;
use super::{ArchiveEngine, ArchiveEntry};
use crate::fuse::FuseMountManager;
use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct ArchiveManager {
    current_path: Option<PathBuf>,
    all_entries: Vec<ArchiveEntry>,
    current_folder: PathBuf,
    history_back: Vec<PathBuf>,
    history_forward: Vec<PathBuf>,
    active_engine_index: usize,
    /// FUSE-монтирование текущего архива.
    /// None если libfuse3 недоступен (graceful fallback на извлечение в tmpfs).
    fuse_mount: Option<FuseMountManager>,
}

impl ArchiveManager {
    pub fn new() -> Self {
        Self {
            current_path: None,
            all_entries: Vec::new(),
            current_folder: PathBuf::new(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            active_engine_index: 0,
            fuse_mount: None,
        }
    }

    pub fn current_path(&self) -> Option<&Path> {
        self.current_path.as_deref()
    }

    pub fn current_folder(&self) -> &Path {
        &self.current_folder
    }

    /// Возвращает корень FUSE-монтирования для текущего архива.
    /// Используется drag_source для мгновенной передачи путей без распаковки.
    pub fn fuse_mount_path(&self) -> Option<&Path> {
        self.fuse_mount.as_ref().map(|m| m.mount_path())
    }

    pub fn enter_folder(&mut self, folder: PathBuf) {
        if self.current_folder != folder {
            tracing::debug!("Entering folder: {}", folder.display());
            self.history_back.push(self.current_folder.clone());
            self.history_forward.clear();
            self.current_folder = folder;
        }
    }

    pub fn set_current_folder(&mut self, folder: PathBuf) {
        self.enter_folder(folder);
    }

    pub fn navigate_back(&mut self) -> bool {
        if let Some(prev) = self.history_back.pop() {
            tracing::debug!("Navigating back to folder: {}", prev.display());
            self.history_forward.push(self.current_folder.clone());
            self.current_folder = prev;
            true
        } else if !self.current_folder.as_os_str().is_empty() {
            let current = self.current_folder.clone();
            if self.current_folder.pop() {
                self.history_forward.push(current);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn navigate_forward(&mut self) -> bool {
        if let Some(next) = self.history_forward.pop() {
            tracing::debug!("Navigating forward to folder: {}", next.display());
            self.history_back.push(self.current_folder.clone());
            self.current_folder = next;
            true
        } else {
            false
        }
    }

    pub fn visible_entries(&self) -> Vec<ArchiveEntry> {
        let folder = &self.current_folder;
        let mut dir_map: BTreeMap<String, ArchiveEntry> = BTreeMap::new();
        let mut file_entries: Vec<ArchiveEntry> = Vec::new();

        for entry in &self.all_entries {
            let relative = if folder.as_os_str().is_empty() {
                entry.path.clone()
            } else if let Ok(rel) = entry.path.strip_prefix(folder) {
                rel.to_path_buf()
            } else {
                continue;
            };

            if relative.as_os_str().is_empty() {
                continue;
            }

            let mut components = relative.components();
            if let Some(first) = components.next() {
                let name = first.as_os_str().to_string_lossy().to_string();
                let is_direct_file = components.next().is_none() && !entry.is_dir;

                if is_direct_file {
                    file_entries.push(entry.clone());
                } else {
                    dir_map.entry(name.clone()).or_insert_with(|| {
                        let folder_path = if folder.as_os_str().is_empty() {
                            PathBuf::from(&name)
                        } else {
                            folder.join(&name)
                        };
                        ArchiveEntry::new(folder_path, 0, true)
                    });
                }
            }
        }

        let mut result = Vec::new();
        for (_, dir_entry) in dir_map {
            result.push(dir_entry);
        }
        result.extend(file_entries);

        result
    }

    fn get_ordered_engines(&self, path: &Path) -> Vec<Box<dyn ArchiveEngine>> {
        let detected = FormatDetector::detect(path);
        let mut engines: Vec<Box<dyn ArchiveEngine>> = Vec::new();

        if let Some(format) = detected {
            let primary: Box<dyn ArchiveEngine> = match format {
                DetectedFormat::Zip => Box::new(ZipEngine),
                DetectedFormat::TarGz => Box::new(TarEngine::new(TarCompression::Gzip)),
                DetectedFormat::TarZst => Box::new(TarEngine::new(TarCompression::Zstd)),
                DetectedFormat::Tar => Box::new(TarEngine::new(TarCompression::None)),
            };
            engines.push(primary);
        }

        engines.push(Box::new(LibarchiveEngine));
        engines.push(Box::new(ZipEngine));
        engines.push(Box::new(TarEngine::new(TarCompression::Gzip)));
        engines.push(Box::new(TarEngine::new(TarCompression::Zstd)));
        engines.push(Box::new(TarEngine::new(TarCompression::None)));

        engines
    }

    pub fn get_engine_for_path(path: &Path) -> Result<Box<dyn ArchiveEngine>> {
        let format = FormatDetector::detect(path);
        match format {
            Some(DetectedFormat::Zip) => Ok(Box::new(ZipEngine)),
            Some(DetectedFormat::TarGz) => Ok(Box::new(TarEngine::new(TarCompression::Gzip))),
            Some(DetectedFormat::TarZst) => Ok(Box::new(TarEngine::new(TarCompression::Zstd))),
            Some(DetectedFormat::Tar) => Ok(Box::new(TarEngine::new(TarCompression::None))),
            // Неизвестный формат — пробуем LibarchiveEngine (поддерживает 7z, rar, xz, bz2, ...)
            None => {
                tracing::warn!("Unknown archive format for {:?}, trying LibarchiveEngine", path);
                Ok(Box::new(LibarchiveEngine))
            }
        }
    }

    pub fn open_archive(&mut self, path: &Path) -> Result<Vec<ArchiveEntry>> {
        self.open_archive_with_password(path, None)
    }

    pub fn open_archive_with_password(&mut self, path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>> {
        tracing::info!("Attempting to open archive: {} (password: {})", path.display(), password.is_some());

        let engines = self.get_ordered_engines(path);
        let mut last_err = anyhow!("Failed to open archive: Unsupported format or corrupted file");

        for (i, engine) in engines.into_iter().enumerate() {
            let res = engine.list_entries_with_password(path, password);
            match res {
                Ok(entries) => {
                    tracing::info!(
                        "Successfully opened archive {} using engine option #{}. Total entries: {}",
                        path.display(),
                        i,
                        entries.len()
                    );
                    self.current_path = Some(path.to_path_buf());
                    self.all_entries = entries;
                    self.current_folder = PathBuf::new();
                    self.history_back.clear();
                    self.history_forward.clear();
                    self.active_engine_index = i;

                    self.fuse_mount = None;
                    match FuseMountManager::mount(path.to_path_buf(), &self.all_entries) {
                        Ok(mount) => {
                            tracing::info!(
                                "FUSE: virtual FS ready at '{}' — DnD will use zero-latency streaming",
                                mount.mount_path().display()
                            );
                            self.fuse_mount = Some(mount);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "FUSE mount unavailable (libfuse3 not installed?): {}. \
                                 DnD will fallback to tmpfs extraction.",
                                e
                            );
                        }
                    }

                    return Ok(self.visible_entries());
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    if err_msg.contains("Password required") || err_msg.contains("encrypted") {
                        return Err(err);
                    }
                    tracing::warn!("Engine option #{} failed for {}: {:?}", i, path.display(), err);
                    last_err = err;
                }
            }
        }

        tracing::error!("All engine attempts failed for {}: {:?}", path.display(), last_err);
        Err(last_err)
    }

    pub fn create_archive(&mut self, archive_path: &Path, files: &[PathBuf]) -> Result<()> {
        tracing::info!(
            "Creating archive {} with {} source file(s)",
            archive_path.display(),
            files.len()
        );
        let engine = Self::get_engine_for_path(archive_path)?;
        engine.create_archive(archive_path, files)?;
        self.open_archive(archive_path)?;
        Ok(())
    }

    pub fn extract_selected(
        &self,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
    ) -> Result<Vec<PathBuf>> {
        let current = self
            .current_path
            .as_ref()
            .ok_or_else(|| anyhow!("No archive is currently open"))?;

        tracing::info!(
            "Extracting {} selected entry(ies) from {} into {}",
            entry_paths.len(),
            current.display(),
            dest_dir.display()
        );

        let engines = self.get_ordered_engines(current);

        // 1. Приоритет — движок, который успешно открыл архив
        if let Some(engine) = engines.get(self.active_engine_index) {
            if let Ok(extracted) = engine.extract_entries(current, entry_paths, dest_dir) {
                if !extracted.is_empty() {
                    return Ok(extracted);
                }
            }
        }

        // 2. Перебираем все остальные движки
        for (i, engine) in engines.iter().enumerate() {
            if i == self.active_engine_index {
                continue; // Уже пробовали
            }
            if let Ok(extracted) = engine.extract_entries(current, entry_paths, dest_dir) {
                if !extracted.is_empty() {
                    return Ok(extracted);
                }
            }
        }

        Err(anyhow!("Failed to extract selected entries from archive"))
    }

    pub fn extract_selected_with_password(
        &self,
        entry_paths: &[PathBuf],
        dest_dir: &Path,
        password: Option<&str>,
    ) -> Result<Vec<PathBuf>> {
        let current = self
            .current_path
            .as_ref()
            .ok_or_else(|| anyhow!("No archive is currently open"))?;
        tracing::info!(
            "Extracting selected entries from {} to {} (password: {})",
            current.display(),
            dest_dir.display(),
            password.is_some()
        );

        let engines = self.get_ordered_engines(current);

        if let Some(engine) = engines.get(self.active_engine_index) {
            if let Ok(extracted) = engine.extract_entries_with_password(current, entry_paths, dest_dir, password) {
                if !extracted.is_empty() {
                    return Ok(extracted);
                }
            }
        }

        for (i, engine) in engines.iter().enumerate() {
            if i == self.active_engine_index {
                continue;
            }
            if let Ok(extracted) = engine.extract_entries_with_password(current, entry_paths, dest_dir, password) {
                if !extracted.is_empty() {
                    return Ok(extracted);
                }
            }
        }

        Err(anyhow!("Failed to extract selected entries from archive"))
    }

    pub fn delete_selected_entries(&mut self, entry_paths: &[PathBuf]) -> Result<()> {
        let current = self
            .current_path
            .as_ref()
            .ok_or_else(|| anyhow!("No archive is currently open"))?;
        let saved_folder = self.current_folder.clone();
        tracing::info!(
            "Deleting {} selected entry(ies) from {}",
            entry_paths.len(),
            current.display()
        );

        let engines = self.get_ordered_engines(current);
        let engine = engines
            .get(self.active_engine_index)
            .ok_or_else(|| anyhow!("No active engine available for deleting entries"))?;

        engine.delete_entries(current, entry_paths)?;
        let path = current.clone();
        self.open_archive(&path)?;

        if !saved_folder.as_os_str().is_empty() && !self.all_entries.iter().any(|e| e.path.starts_with(&saved_folder)) {
            let mut parent = saved_folder;
            parent.pop();
            self.current_folder = parent;
        } else {
            self.current_folder = saved_folder;
        }

        Ok(())
    }

    pub fn verify_integrity(&self, password: Option<&str>) -> Result<super::IntegrityReport> {
        let current = self
            .current_path
            .as_ref()
            .ok_or_else(|| anyhow!("No archive is currently open"))?;
        let engines = self.get_ordered_engines(current);
        let engine = engines
            .get(self.active_engine_index)
            .ok_or_else(|| anyhow!("No active engine available for integrity check"))?;

        engine.verify_integrity(current, password)
    }

    pub fn add_files(&mut self, files: &[PathBuf]) -> Result<()> {
        let current = self
            .current_path
            .as_ref()
            .ok_or_else(|| anyhow!("No archive is open to add files to"))?;
        let saved_folder = self.current_folder.clone();
        tracing::info!("Adding {} file(s) to open archive {}", files.len(), current.display());
        let engines = self.get_ordered_engines(current);
        let engine = engines
            .get(self.active_engine_index)
            .ok_or_else(|| anyhow!("No active engine available for adding entries"))?;
        engine.add_entries(current, files)?;
        let path = current.clone();
        self.open_archive(&path)?;
        self.current_folder = saved_folder;
        Ok(())
    }

    pub fn encrypt_archive(&mut self, password: &str) -> Result<()> {
        let current = self
            .current_path
            .as_ref()
            .ok_or_else(|| anyhow!("No archive is currently open"))?;
        let saved_folder = self.current_folder.clone();
        tracing::info!("Encrypting open archive {}", current.display());

        let engines = self.get_ordered_engines(current);
        let engine = engines
            .get(self.active_engine_index)
            .ok_or_else(|| anyhow!("No active engine available for encryption"))?;

        engine.encrypt_archive(current, password)?;
        let path = current.clone();
        self.open_archive_with_password(&path, Some(password))?;
        self.current_folder = saved_folder;
        Ok(())
    }
}
