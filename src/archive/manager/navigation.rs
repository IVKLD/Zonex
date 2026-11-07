use crate::archive::ArchiveEntry;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Default)]
pub struct NavigationState {
    pub current_folder: PathBuf,
    pub history_back: Vec<PathBuf>,
    pub history_forward: Vec<PathBuf>,
    cached_visible: RefCell<HashMap<PathBuf, Vec<ArchiveEntry>>>,
}

impl NavigationState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn enter_folder(&mut self, folder: PathBuf) {
        if self.current_folder != folder {
            tracing::debug!("Entering folder: {}", folder.display());
            self.history_back.push(self.current_folder.clone());
            self.history_forward.clear();
            self.current_folder = folder;
        }
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

    pub fn visible_entries(&self, all_entries: &[ArchiveEntry]) -> Vec<ArchiveEntry> {
        let folder = &self.current_folder;
        if let Some(cached) = self.cached_visible.borrow().get(folder) {
            return cached.clone();
        }

        let mut dir_map: BTreeMap<String, ArchiveEntry> = BTreeMap::new();
        let mut file_entries: Vec<ArchiveEntry> = Vec::with_capacity(all_entries.len());

        for entry in all_entries {
            let relative: &std::path::Path = if folder.as_os_str().is_empty() {
                &entry.path
            } else if let Ok(rel) = entry.path.strip_prefix(folder) {
                rel
            } else {
                continue;
            };

            if relative.as_os_str().is_empty() {
                continue;
            }

            let mut components = relative.components();
            if let Some(first) = components.next() {
                let name = first.as_os_str().to_string_lossy();
                let is_direct_file = components.next().is_none() && !entry.is_dir;

                if is_direct_file {
                    file_entries.push(entry.clone());
                } else {
                    dir_map.entry(name.into_owned()).or_insert_with_key(|key| {
                        let folder_path = if folder.as_os_str().is_empty() {
                            PathBuf::from(key)
                        } else {
                            folder.join(key)
                        };
                        ArchiveEntry::new(folder_path, 0, true)
                    });
                }
            }
        }

        let mut result = Vec::with_capacity(dir_map.len() + file_entries.len());
        result.extend(dir_map.into_values());
        result.extend(file_entries);

        self.cached_visible
            .borrow_mut()
            .insert(folder.clone(), result.clone());

        result
    }
}
