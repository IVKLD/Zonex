use crate::archive::ArchiveEntry;

pub fn get_icon_name_for_entry(entry: &ArchiveEntry) -> &'static str {
    if entry.is_dir {
        return "folder-symbolic";
    }

    let ext = match entry.path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext,
        None => return "text-x-generic-symbolic",
    };

    let mut buf = [0u8; 16];
    if ext.len() > buf.len() {
        return "text-x-generic-symbolic";
    }
    let bytes = &mut buf[..ext.len()];
    bytes.copy_from_slice(ext.as_bytes());
    bytes.make_ascii_lowercase();

    match &*bytes {
        b"zip" | b"tar" | b"gz" | b"bz2" | b"xz" | b"zst" | b"7z" | b"rar" | b"iso" | b"deb"
        | b"rpm" => "package-x-generic-symbolic",

        b"png" | b"jpg" | b"jpeg" | b"gif" | b"svg" | b"webp" | b"bmp" | b"ico" => {
            "image-x-generic-symbolic"
        }

        b"rs" | b"py" | b"c" | b"cpp" | b"h" | b"hpp" | b"js" | b"ts" | b"json" | b"toml"
        | b"yaml" | b"yml" | b"sh" | b"nix" => "text-x-script-symbolic",

        b"mp4" | b"mkv" | b"avi" | b"webm" | b"mov" | b"flv" => "video-x-generic-symbolic",

        b"mp3" | b"flac" | b"wav" | b"ogg" | b"m4a" | b"aac" => "audio-x-generic-symbolic",

        b"exe" | b"bin" | b"so" | b"dll" | b"appimage" => "application-x-executable-symbolic",

        _ => "text-x-generic-symbolic",
    }
}
