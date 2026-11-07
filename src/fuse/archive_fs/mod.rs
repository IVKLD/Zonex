mod filesystem;
mod io;
mod nodes;
mod streaming;
mod tree;

use crate::archive::ArchiveEntry;
use fuser::FileAttr;
use nodes::{make_attr, FuseNode};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use streaming::{start_streaming, SharedBuffer};

pub struct ArchiveFuseFs {
    archive_path: PathBuf,
    nodes: HashMap<u64, FuseNode>,
    stream_buffers: Mutex<HashMap<u64, SharedBuffer>>,
}

impl ArchiveFuseFs {
    pub fn new(archive_path: PathBuf, entries: &[ArchiveEntry]) -> Self {
        let nodes = tree::build(entries);
        Self {
            archive_path,
            nodes,
            stream_buffers: Mutex::new(HashMap::new()),
        }
    }

    fn start_streaming(&self, ino: u64) -> SharedBuffer {
        let archive_entry_path = self
            .nodes
            .get(&ino)
            .and_then(|n| n.archive_entry_path.clone());
        start_streaming(
            ino,
            self.archive_path.clone(),
            archive_entry_path,
            &self.stream_buffers,
        )
    }

    pub fn find_node_ino(&self, parent: u64, name: &str) -> Option<u64> {
        let p_node = self.nodes.get(&parent)?;
        p_node.children.iter().find_map(|&child_ino| {
            self.nodes
                .get(&child_ino)
                .filter(|n| n.name == name)
                .map(|n| n.ino)
        })
    }

    pub fn make_attr(&self, ino: u64) -> Option<FileAttr> {
        let node = self.nodes.get(&ino)?;
        let dynamic_size = if node.size == 0 {
            self.stream_buffers.lock().ok().and_then(|sb| {
                sb.get(&ino).and_then(|sb| {
                    sb.0.lock()
                        .ok()
                        .map(|b| if b.done { b.data.len() as u64 } else { 0 })
                })
            })
        } else {
            None
        };

        Some(make_attr(node, dynamic_size))
    }
}
