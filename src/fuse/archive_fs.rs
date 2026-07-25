use crate::archive::ArchiveEntry;
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    ReplyOpen, Request, FUSE_ROOT_ID,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

// ─────────────────────────────────────────────────────────────────────────────
// Streaming-кэш файла
//
// Принцип работы без блокировки UI:
//   1. open()  → запускает фоновый поток, который извлекает файл и буферизует байты
//               → возвращает МГНОВЕННО (не блокирует FM)
//   2. read()  → читает из буфера; если данных пока нет — ждёт через Condvar
//               → происходит в FUSE-потоке, GTK не затрагивается
//   3. Nautilus видит правильный размер из getattr (из заголовка архива),
//              получает байты по мере извлечения → прогресс-бар плавный.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct StreamBuffer {
    /// Накопленные байты (пишет фоновый поток, читает FUSE-поток)
    data: Vec<u8>,
    /// true = фоновый поток завершил запись (успешно или с ошибкой)
    done: bool,
    /// Ошибка извлечения, если была
    error: Option<String>,
}

/// Разделяемый буфер для одного файла: фоновый поток пишет, FUSE-поток читает
type SharedBuffer = Arc<(Mutex<StreamBuffer>, Condvar)>;

/// Узел FUSE-дерева (файл или директория)
#[derive(Clone)]
struct FuseNode {
    ino: u64,
    parent: u64,
    name: String,
    kind: FileType,
    /// Для файлов: путь к записи внутри архива
    archive_entry_path: Option<PathBuf>,
    /// Размер файла из заголовка архива.
    /// Критично: без правильного size FM не показывает прогресс-бар.
    /// После streaming-извлечения обновляется реальным размером.
    size: u64,
    children: Vec<u64>,
}

/// FUSE-файловая система для архива.
/// Работает в BackgroundSession (отдельный поток).
/// GTK event loop НЕ блокируется ни при каких обстоятельствах.
pub struct ArchiveFuseFs {
    archive_path: PathBuf,
    nodes: HashMap<u64, FuseNode>,
    next_ino: u64,
    /// Streaming-буферы: inode → SharedBuffer (созданные в open(), читаемые в read())
    stream_buffers: HashMap<u64, SharedBuffer>,
}

impl ArchiveFuseFs {
    pub fn new(archive_path: PathBuf, entries: &[ArchiveEntry]) -> Self {
        let mut fs = Self {
            archive_path,
            nodes: HashMap::new(),
            next_ino: FUSE_ROOT_ID + 1,
            stream_buffers: HashMap::new(),
        };

        fs.nodes.insert(
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
            fs.insert_entry(entry);
        }

        fs
    }

    fn insert_entry(&mut self, entry: &ArchiveEntry) {
        let components: Vec<String> = entry
            .path
            .components()
            .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
            .collect();

        if components.is_empty() {
            return;
        }

        let mut parent_ino = FUSE_ROOT_ID;

        for (i, name) in components.iter().enumerate() {
            let is_leaf = i == components.len() - 1;

            if is_leaf && !entry.is_dir {
                let ino = self.next_ino;
                self.next_ino += 1;

                self.nodes.insert(
                    ino,
                    FuseNode {
                        ino,
                        parent: parent_ino,
                        name: name.clone(),
                        kind: FileType::RegularFile,
                        archive_entry_path: Some(entry.path.clone()),
                        size: entry.size,
                        children: Vec::new(),
                    },
                );
                self.nodes
                    .get_mut(&parent_ino)
                    .expect("parent inode must exist")
                    .children
                    .push(ino);
            } else {
                let existing_ino = {
                    let parent = self.nodes.get(&parent_ino).expect("parent must exist");
                    parent.children.iter().find_map(|&child_ino| {
                        self.nodes.get(&child_ino).and_then(|n| {
                            if n.name == *name && n.kind == FileType::Directory {
                                Some(child_ino)
                            } else {
                                None
                            }
                        })
                    })
                };

                if let Some(ino) = existing_ino {
                    parent_ino = ino;
                } else {
                    let ino = self.next_ino;
                    self.next_ino += 1;

                    self.nodes.insert(
                        ino,
                        FuseNode {
                            ino,
                            parent: parent_ino,
                            name: name.clone(),
                            kind: FileType::Directory,
                            archive_entry_path: None,
                            size: 0,
                            children: Vec::new(),
                        },
                    );
                    self.nodes
                        .get_mut(&parent_ino)
                        .expect("parent inode must exist")
                        .children
                        .push(ino);
                    parent_ino = ino;
                }
            }
        }
    }

    /// Запускает фоновый поток извлечения файла.
    /// open() возвращает мгновенно; данные появляются в SharedBuffer по мере извлечения.
    fn start_streaming(&mut self, ino: u64) -> SharedBuffer {
        // Если стрим уже запущен (или файл уже извлечён) — возвращаем буфер
        if let Some(existing) = self.stream_buffers.get(&ino) {
            return existing.clone();
        }

        let archive_entry_path = self
            .nodes
            .get(&ino)
            .and_then(|n| n.archive_entry_path.clone());

        let shared: SharedBuffer = Arc::new((Mutex::new(StreamBuffer::default()), Condvar::new()));
        self.stream_buffers.insert(ino, shared.clone());

        let archive_path = self.archive_path.clone();

        let Some(entry_path) = archive_entry_path else {
            // Директория или запись без пути — помечаем как done сразу
            {
                let (lock, cvar) = &*shared;
                let mut buf = lock.lock().unwrap();
                buf.done = true;
                cvar.notify_all();
            }
            return shared;
        };

        let shared_clone = shared.clone();
        std::thread::spawn(move || {
            let path_str = entry_path.to_string_lossy().to_string();
            tracing::debug!("FUSE streaming: extracting '{}' in background", path_str);

            let result: Result<Vec<u8>, String> = (|| {
                let source = std::fs::File::open(&archive_path)
                    .map_err(|e| format!("cannot open archive: {e}"))?;
                let mut cursor = std::io::Cursor::new(Vec::new());
                compress_tools::uncompress_archive_file(source, &mut cursor, &path_str)
                    .map_err(|e| format!("uncompress failed: {e}"))?;
                Ok(cursor.into_inner())
            })();

            let (lock, cvar) = &*shared_clone;
            let mut buf = lock.lock().unwrap();
            match result {
                Ok(data) => {
                    tracing::debug!("FUSE streaming: '{}' done — {} bytes", path_str, data.len());
                    buf.data = data;
                    buf.done = true;
                }
                Err(e) => {
                    tracing::error!("FUSE streaming: '{}' failed: {}", path_str, e);
                    buf.error = Some(e);
                    buf.done = true;
                }
            }
            cvar.notify_all();
        });

        shared
    }

    #[allow(dead_code)]
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
        // Если уже извлекали — берём реальный размер из буфера (точнее заголовка)
        let size = if node.size == 0 {
            // Для файлов с size=0 в заголовке (LibarchiveEngine) пробуем буфер
            self.stream_buffers
                .get(&ino)
                .and_then(|sb| {
                    sb.0.lock().ok().map(|b| if b.done { b.data.len() as u64 } else { 0 })
                })
                .unwrap_or(0)
        } else {
            node.size
        };

        let now = SystemTime::now();
        Some(FileAttr {
            ino,
            size,
            blocks: (size + 511) / 512,
            atime: now,
            mtime: now,
            ctime: now,
            crtime: now,
            kind: node.kind,
            perm: if node.kind == FileType::Directory {
                0o755
            } else {
                0o644 // Read-Write for owner (rw-r--r--) so copied files can be edited
            },
            nlink: if node.kind == FileType::Directory { 2 } else { 1 },
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            blksize: 65536,
            flags: 0,
        })
    }
}

const TTL: Duration = Duration::from_secs(1);

impl Filesystem for ArchiveFuseFs {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_string_lossy();

        let found = self.nodes.get(&parent).and_then(|p_node| {
            p_node.children.iter().find_map(|&child_ino| {
                self.nodes
                    .get(&child_ino)
                    .filter(|n| n.name == name_str.as_ref())
                    .map(|n| n.ino)
            })
        });

        match found {
            Some(ino) => {
                if let Some(attr) = self.make_attr(ino) {
                    reply.entry(&TTL, &attr, 0);
                } else {
                    reply.error(libc::ENOENT);
                }
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match self.make_attr(ino) {
            Some(attr) => reply.attr(&TTL, &attr),
            None => reply.error(libc::ENOENT),
        }
    }

    /// КЛЮЧЕВОЕ ИЗМЕНЕНИЕ: open() запускает извлечение в фоне и ВОЗВРАЩАЕТ НЕМЕДЛЕННО.
    /// Файловый менеджер получает ответ за ~0ms и начинает копирование.
    /// Данные поступают из буфера по мере извлечения.
    fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        // Запускаем фоновое извлечение (или возвращаем уже запущенный буфер)
        self.start_streaming(ino);
        // Возвращаем МГНОВЕННО — никакого ожидания!
        // DIRECT_IO: говорим ядру не кэшировать страницы (мы сами управляем данными)
        reply.opened(0, fuser::consts::FOPEN_DIRECT_IO);
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let offset = offset as usize;
        let end = offset + size as usize;

        // Получаем или создаём SharedBuffer (на случай если open() не был вызван)
        let shared = if let Some(sb) = self.stream_buffers.get(&ino) {
            sb.clone()
        } else {
            self.start_streaming(ino)
        };

        let (lock, cvar) = &*shared;

        // Ждём пока данных станет достаточно или извлечение завершится
        // Используем wait_timeout чтобы не зависнуть навечно
        let buf = lock.lock().unwrap();
        let buf = cvar
            .wait_timeout_while(buf, Duration::from_secs(30), |b| {
                // Продолжаем ждать если: не завершено И данных меньше чем нужно
                !b.done && b.data.len() < end
            })
            .unwrap()
            .0;

        if let Some(ref e) = buf.error {
            tracing::error!("FUSE read: streaming error for ino {}: {}", ino, e);
            reply.error(libc::EIO);
            return;
        }

        if offset >= buf.data.len() {
            // Запрошенный offset за пределами данных — нормальный EOF
            reply.data(&[]);
        } else {
            let actual_end = end.min(buf.data.len());
            reply.data(&buf.data[offset..actual_end]);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let (self_ino, parent_ino, children_snapshot) = match self.nodes.get(&ino) {
            Some(node) => (node.ino, node.parent, node.children.clone()),
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let mut all_entries: Vec<(u64, FileType, String)> = vec![
            (self_ino, FileType::Directory, ".".to_string()),
            (parent_ino, FileType::Directory, "..".to_string()),
        ];

        for child_ino in children_snapshot {
            if let Some(child) = self.nodes.get(&child_ino) {
                all_entries.push((child_ino, child.kind, child.name.clone()));
            }
        }

        for (i, (child_ino, kind, name)) in all_entries.iter().enumerate().skip(offset as usize) {
            if reply.add(*child_ino, (i + 1) as i64, *kind, name) {
                break;
            }
        }

        reply.ok();
    }

    fn opendir(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        if self.nodes.contains_key(&ino) {
            reply.opened(0, 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn statfs(&mut self, _req: &Request, _ino: u64, reply: fuser::ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, 512, 255, 0);
    }
}
