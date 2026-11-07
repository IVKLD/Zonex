use crate::archive::ArchiveEntry;

pub fn filter_and_sort(all: &[ArchiveEntry], query: &str, sort_idx: u32) -> Vec<ArchiveEntry> {
    let mut filtered: Vec<ArchiveEntry> = if query.is_empty() {
        all.to_vec()
    } else {
        all.iter()
            .filter(|e| e.file_name().to_lowercase().contains(query))
            .cloned()
            .collect()
    };

    match sort_idx {
        0 => {
            filtered.sort_by_cached_key(|e| (!e.is_dir, e.file_name().to_lowercase()));
        }
        1 => {
            filtered.sort_by_cached_key(|e| e.file_name().to_lowercase());
        }
        2 => {
            filtered.sort_by_cached_key(|e| std::cmp::Reverse(e.file_name().to_lowercase()));
        }
        3 => {
            filtered.sort_by_key(|b| std::cmp::Reverse(b.size));
        }
        4 => {
            filtered.sort_by_key(|a| a.size);
        }
        _ => {}
    }

    filtered
}
