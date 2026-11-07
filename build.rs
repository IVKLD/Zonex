fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "linux" {
        let rustflags = std::env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
        if rustflags.contains("X11") || rustflags.contains("x11") || rustflags.contains("xcb") {
            panic!("Build failed: X11 dependencies are strictly prohibited.");
        }
    }
}
