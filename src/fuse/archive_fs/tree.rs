use super::nodes::{FuseNode, FUSE_ROOT_ID};
use crate::archive::ArchiveEntry;
use fuser::FileType;
use std::collections::HashMap;

pub fn build(entries: &[ArchiveEntry]) -> HashMap<u64, FuseNode> {
    let mut nodes = HashMap::with_capacity(entries.len() + 1);
    let mut dir_lookup = HashMap::new();
    let mut next_ino = FUSE_ROOT_ID + 1;

    nodes.insert(
        FUSE_ROOT_ID,
        FuseNode {
            ino: FUSE_ROOT_ID,
            parent: FUSE_ROOT_ID,
            name: String::new(),
            kind: FileType::Directory,
            archive_entry_path: None,
            size: 0,
            children: Vec::new(),
        },
    );

    for entry in entries {
        if entry.path.as_os_str().is_empty() {
            continue;
        }
        insert_entry(&mut nodes, &mut dir_lookup, &mut next_ino, entry);
    }

    nodes
}

fn insert_entry<'a>(
    nodes: &mut HashMap<u64, FuseNode>,
    dir_lookup: &mut HashMap<(u64, &'a str), u64>,
    next_ino: &mut u64,
    entry: &'a ArchiveEntry,
) {
    let components: Vec<&str> = entry
        .path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();

    if components.is_empty() {
        return;
    }

    let mut parent_ino = FUSE_ROOT_ID;

    for (i, &name) in components.iter().enumerate() {
        let is_leaf = i == components.len() - 1;

        if is_leaf && !entry.is_dir {
            let ino = *next_ino;
            *next_ino += 1;

            nodes.insert(
                ino,
                FuseNode {
                    ino,
                    parent: parent_ino,
                    name: name.to_string(),
                    kind: FileType::RegularFile,
                    archive_entry_path: Some(entry.path.clone()),
                    size: entry.size,
                    children: Vec::new(),
                },
            );
            nodes
                .get_mut(&parent_ino)
                .expect("parent inode must exist")
                .children
                .push(ino);
        } else if let Some(&existing_ino) = dir_lookup.get(&(parent_ino, name)) {
            parent_ino = existing_ino;
        } else {
            let ino = *next_ino;
            *next_ino += 1;

            nodes.insert(
                ino,
                FuseNode {
                    ino,
                    parent: parent_ino,
                    name: name.to_string(),
                    kind: FileType::Directory,
                    archive_entry_path: None,
                    size: 0,
                    children: Vec::new(),
                },
            );
            nodes
                .get_mut(&parent_ino)
                .expect("parent inode must exist")
                .children
                .push(ino);
            dir_lookup.insert((parent_ino, name), ino);
            parent_ino = ino;
        }
    }
}
