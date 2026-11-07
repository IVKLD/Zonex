use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
pub struct StreamBuffer {
    pub data: Vec<u8>,
    pub done: bool,
    pub error: Option<String>,
}

pub type SharedBuffer = Arc<(Mutex<StreamBuffer>, Condvar)>;

pub fn start_streaming(
    ino: u64,
    archive_path: PathBuf,
    archive_entry_path: Option<PathBuf>,
    stream_buffers: &Mutex<HashMap<u64, SharedBuffer>>,
) -> SharedBuffer {
    let mut buffers = stream_buffers.lock().unwrap();

    if let Some(existing) = buffers.get(&ino) {
        return existing.clone();
    }

    let shared: SharedBuffer = Arc::new((Mutex::new(StreamBuffer::default()), Condvar::new()));
    buffers.insert(ino, shared.clone());
    drop(buffers);

    let Some(entry_path) = archive_entry_path else {
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
