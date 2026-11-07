use super::context::ActionsContext;
use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use gio::{Cancellable, SimpleAction};
use gtk4::prelude::*;
use gtk4::FileDialog;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn init_empty_archive(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let name = path.to_string_lossy().to_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        let file = File::create(path)?;
        let mut enc = GzEncoder::new(file, Compression::fast());
        Write::write_all(&mut enc, &[0u8; 1024])?;
        enc.finish()?;
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        let file = File::create(path)?;
        let mut enc = zstd::Encoder::new(file, 1)?.auto_finish();
        Write::write_all(&mut enc, &[0u8; 1024])?;
    } else if name.ends_with(".tar") {
        let mut file = File::create(path)?;
        Write::write_all(&mut file, &[0u8; 1024])?;
    } else {
        let file = File::create(path)?;
        let zip = zip::ZipWriter::new(file);
        zip.finish()?;
    }
    Ok(())
}

pub fn register(ctx: &ActionsContext) {
    let window = ctx.window.clone();
    let do_open = ctx.do_open.clone();
    let show_toast = ctx.show_toast.clone();

    let action = SimpleAction::new("new_archive", None);
    action.connect_activate(move |_, _| {
        let dialog = FileDialog::builder()
            .title("Create New Archive")
            .initial_name("archive.zip")
            .build();

        let do_open_c = do_open.clone();
        let toast_c = show_toast.clone();
        dialog.save(Some(&window), Cancellable::NONE, move |res| {
            if let Some(path) = res.ok().and_then(|f| f.path()) {
                let path_task = path.clone();
                let do_open_cb = do_open_c.clone();
                let toast_cb = toast_c.clone();
                let path_cb = path.clone();

                glib::MainContext::default().spawn_local(async move {
                    let res = gio::spawn_blocking(move || {
                        init_empty_archive(&path_task).map_err(|e| e.to_string())
                    })
                    .await
                    .unwrap_or_else(|_| Err("Archive creation panicked".to_string()));

                    match res {
                        Ok(()) => {
                            do_open_cb(path_cb.clone());
                            toast_cb(&format!(
                                "Created new empty archive: {}",
                                path_cb.display()
                            ));
                        }
                        Err(err) => {
                            toast_cb(&format!("Failed to create archive: {}", err));
                        }
                    }
                });
            }
        });
    });
    ctx.window.add_action(&action);
}
