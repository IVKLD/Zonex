use super::ArchiveFuseFs;
use fuser::{Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request};
use std::ffi::OsStr;
use std::time::Duration;

const TTL: Duration = Duration::from_secs(3600);

impl Filesystem for ArchiveFuseFs {
    fn lookup(&self, _req: &Request, parent: fuser::INodeNo, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_string_lossy();

        let found = self.nodes.get(&parent.0).and_then(|p| {
            p.children.iter().copied().find(|&c| {
                self.nodes
                    .get(&c)
                    .is_some_and(|n| n.name == name_str.as_ref())
            })
        });

        match found.and_then(|ino| self.make_attr(ino)) {
            Some(attr) => reply.entry(&TTL, &attr, fuser::Generation(0)),
            None => reply.error(fuser::Errno::from_i32(libc::ENOENT)),
        }
    }

    fn getattr(
        &self,
        _req: &Request,
        ino: fuser::INodeNo,
        _fh: Option<fuser::FileHandle>,
        reply: ReplyAttr,
    ) {
        match self.make_attr(ino.0) {
            Some(attr) => reply.attr(&TTL, &attr),
            None => reply.error(fuser::Errno::from_i32(libc::ENOENT)),
        }
    }

    fn open(
        &self,
        _req: &Request,
        ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: ReplyOpen,
    ) {
        self.start_streaming(ino.0);
        reply.opened(fuser::FileHandle(0), fuser::FopenFlags::FOPEN_DIRECT_IO);
    }

    fn read(
        &self,
        _req: &Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        size: u32,
        _flags: fuser::OpenFlags,
        _lock_owner: Option<fuser::LockOwner>,
        reply: ReplyData,
    ) {
        self.handle_read(ino.0, offset, size, reply);
    }

    fn readdir(
        &self,
        _req: &Request,
        ino: fuser::INodeNo,
        _fh: fuser::FileHandle,
        offset: u64,
        reply: ReplyDirectory,
    ) {
        self.handle_readdir(ino.0, offset, reply);
    }

    fn opendir(
        &self,
        _req: &Request,
        ino: fuser::INodeNo,
        _flags: fuser::OpenFlags,
        reply: ReplyOpen,
    ) {
        if self.nodes.contains_key(&ino.0) {
            reply.opened(fuser::FileHandle(0), fuser::FopenFlags::empty());
        } else {
            reply.error(fuser::Errno::from_i32(libc::ENOENT));
        }
    }

    fn statfs(&self, _req: &Request, _ino: fuser::INodeNo, reply: fuser::ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }
}
