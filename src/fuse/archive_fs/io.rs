use super::ArchiveFuseFs;
use fuser::{FileType, ReplyData, ReplyDirectory};
use std::time::Duration;

impl ArchiveFuseFs {
    pub(crate) fn handle_read(&self, ino: u64, offset: u64, size: u32, reply: ReplyData) {
        let offset = offset as usize;
        let end = offset.saturating_add(size as usize);

        let shared = self.start_streaming(ino);
        let (lock, cvar) = &*shared;

        let buf = lock.lock().unwrap();
        let buf = cvar
            .wait_timeout_while(buf, Duration::from_secs(30), |b| {
                !b.done && b.data.len() < end
            })
            .unwrap()
            .0;

        if let Some(ref e) = buf.error {
            tracing::error!("FUSE read: streaming error for ino {}: {}", ino, e);
            reply.error(fuser::Errno::from_i32(libc::EIO));
            return;
        }

        if !buf.done && buf.data.len() < end {
            tracing::error!("FUSE read: streaming timeout for ino {}", ino);
            reply.error(fuser::Errno::from_i32(libc::ETIMEDOUT));
            return;
        }

        if offset >= buf.data.len() {
            reply.data(&[]);
        } else {
            let actual_end = end.min(buf.data.len());
            reply.data(&buf.data[offset..actual_end]);
        }
    }

    pub(crate) fn handle_readdir(&self, ino: u64, offset: u64, mut reply: ReplyDirectory) {
        let node = match self.nodes.get(&ino) {
            Some(node) => node,
            None => {
                reply.error(fuser::Errno::from_i32(libc::ENOENT));
                return;
            }
        };

        let mut all_entries: Vec<(u64, FileType, &str)> = vec![
            (node.ino, FileType::Directory, "."),
            (node.parent, FileType::Directory, ".."),
        ];

        for child_ino in &node.children {
            if let Some(child) = self.nodes.get(child_ino) {
                all_entries.push((*child_ino, child.kind, &child.name));
            }
        }

        for (i, (child_ino, kind, name)) in all_entries.iter().enumerate().skip(offset as usize) {
            if reply.add(fuser::INodeNo(*child_ino), (i + 1) as u64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }
}
