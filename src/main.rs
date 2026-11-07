use std::path::{Path, PathBuf};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use zonex::app::ZonexApplication;
use zonex::archive::ArchiveManager;

use glib::ExitCode;

fn enforce_wayland_only() {
    std::env::set_var("GDK_BACKEND", "wayland");

    match (
        std::env::var("XDG_RUNTIME_DIR"),
        std::env::var("WAYLAND_DISPLAY"),
    ) {
        (Ok(dir), Ok(display)) if Path::new(&dir).join(&display).exists() => {}
        (_, Ok(_)) => {}
        _ => {
            eprintln!(
                "FATAL: Native Wayland compositor not found. X11/Xorg/XWayland is not supported."
            );
            std::process::exit(1);
        }
    }
}

fn handle_cli_command(args: &[String]) -> Option<ExitCode> {
    match args.get(1).map(String::as_str) {
        Some("--extract-here") if args.len() >= 3 => {
            let archive_path = Path::new(&args[2]);
            let parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
            let mut mgr = ArchiveManager::default();
            match mgr
                .open_archive(archive_path)
                .and_then(|_| mgr.extract_selected_with_password(&[], parent, None))
            {
                Ok(extracted) => {
                    println!(
                        "Extracted {} file(s) to {}",
                        extracted.len(),
                        parent.display()
                    );
                    Some(ExitCode::SUCCESS)
                }
                Err(e) => {
                    eprintln!("Error extracting archive {}: {e}", archive_path.display());
                    Some(ExitCode::FAILURE)
                }
            }
        }
        Some("--compress") if args.len() >= 3 => {
            let source_paths: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
            let target_name = source_paths[0].file_stem().map_or_else(
                || "archive.zip".to_string(),
                |s| format!("{}.zip", s.to_string_lossy()),
            );
            let target_path = source_paths[0].with_file_name(target_name);

            let mut mgr = ArchiveManager::default();
            match mgr.create_archive(&target_path, &source_paths) {
                Ok(_) => {
                    println!(
                        "Compressed {} item(s) to {}",
                        source_paths.len(),
                        target_path.display()
                    );
                    Some(ExitCode::SUCCESS)
                }
                Err(e) => {
                    eprintln!("Error creating archive {}: {e}", target_path.display());
                    Some(ExitCode::FAILURE)
                }
            }
        }
        _ => None,
    }
}

fn main() -> glib::ExitCode {
    enforce_wayland_only();

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("zonex=debug,info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    let log_file = std::fs::File::create("/tmp/zonex.log").ok();
    let file_layer = log_file.map(|file| {
        tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_ansi(false)
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(file_layer)
        .init();

    tracing::info!("==================================================");
    tracing::info!("Zonex Strictly Wayland-Only Archive Manager Started");
    tracing::info!("Logs are written to stdout and /tmp/zonex.log");
    tracing::info!("==================================================");

    let args: Vec<String> = std::env::args().collect();
    if let Some(code) = handle_cli_command(&args) {
        return code;
    }

    let app = ZonexApplication::new();
    app.run()
}
