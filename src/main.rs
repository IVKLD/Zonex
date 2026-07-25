use zonex::app::ZonexApplication;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

fn enforce_wayland_only() {
    // 3.2 Runtime check: Force GDK_BACKEND=wayland and verify Wayland socket presence
    std::env::set_var("GDK_BACKEND", "wayland");

    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let xdg_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();

    let has_socket = if let (Some(dir), Some(display)) = (&xdg_runtime_dir, &wayland_display) {
        std::path::Path::new(dir).join(display).exists()
    } else {
        wayland_display.is_some()
    };

    if !has_socket {
        eprintln!("FATAL: Native Wayland compositor not found. X11/Xorg/XWayland is not supported.");
        std::process::exit(1);
    }
}

fn main() -> glib::ExitCode {
    enforce_wayland_only();

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("zonex=debug,info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);

    let log_file = std::fs::File::create("/tmp/zonex.log").ok();

    if let Some(file) = log_file {
        let file_layer = tracing_subscriber::fmt::layer()
            .with_writer(file)
            .with_ansi(false);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .with(file_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .init();
    }

    tracing::info!("==================================================");
    tracing::info!("Zonex Strictly Wayland-Only Archive Manager Started");
    tracing::info!("Logs are written to stdout and /tmp/zonex.log");
    tracing::info!("==================================================");

    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--extract-here" {
        use std::path::PathBuf;
        use zonex::archive::ArchiveManager;
        let archive_path = PathBuf::from(&args[2]);
        if let Some(parent) = archive_path.parent() {
            let mut mgr = ArchiveManager::new();
            if let Ok(_) = mgr.open_archive(&archive_path) {
                if let Ok(extracted) = mgr.extract_selected_with_password(&[], parent, None) {
                    println!("Extracted {} file(s) to {}", extracted.len(), parent.display());
                    return glib::ExitCode::SUCCESS;
                }
            }
        }
    } else if args.len() >= 3 && args[1] == "--compress" {
        use std::path::PathBuf;
        use zonex::archive::ArchiveManager;
        let source_paths: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
        let target_name = source_paths[0]
            .file_stem()
            .map(|s| format!("{}.zip", s.to_string_lossy()))
            .unwrap_or_else(|| "archive.zip".to_string());
        let target_path = source_paths[0].with_file_name(target_name);

        let mut mgr = ArchiveManager::new();
        if let Ok(_) = mgr.create_archive(&target_path, &source_paths) {
            println!("Compressed {} item(s) to {}", source_paths.len(), target_path.display());
            return glib::ExitCode::SUCCESS;
        }
    }

    let app = ZonexApplication::new();
    app.run()
}
