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

    let app = ZonexApplication::new();
    app.run()
}
