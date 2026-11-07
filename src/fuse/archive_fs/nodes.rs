use fuser::{FileAttr, FileType};
use std::path::PathBuf;
use std::time::SystemTime;

pub const FUSE_ROOT_ID: u64 = 1;

#[derive(Clone)]
pub struct FuseNode {
    pub ino: u64,
    pub parent: u64,
    pub name: String,
    pub kind: FileType,
    pub archive_entry_path: Option<PathBuf>,
    pub size: u64,
    pub children: Vec<u64>,
}

pub fn make_attr(node: &FuseNode, dynamic_size: Option<u64>) -> FileAttr {
    let size = if node.size == 0 {
        dynamic_size.unwrap_or(0)
    } else {
        node.size
    };

    let now = SystemTime::now();
    FileAttr {
        ino: fuser::INodeNo(node.ino),
        size,
        blocks: size.div_ceil(512),
        atime: now,
        mtime: now,
        ctime: now,
        crtime: now,
        kind: node.kind,
        perm: if node.kind == FileType::Directory {
            0o755
        } else {
            0o644
        },
        nlink: if node.kind == FileType::Directory {
            2
        } else {
            1
        },
        uid: unsafe { libc::getuid() },
        gid: unsafe { libc::getgid() },
        rdev: 0,
        blksize: 65536,
        flags: 0,
    }
}
