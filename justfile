# Zonex - Task Automation Justfile

default: run

# Run the Zonex Wayland Archive Manager
run:
    nix develop --command cargo run

# Build debug binary
build:
    nix develop --command cargo build

# Build release binary
release:
    nix develop --command cargo build --release

# Check code for compilation errors
check:
    nix develop --command cargo check

# Run linter
clippy:
    nix develop --command cargo clippy

# Alias for zed
zeditor: 
    zeditor .

# Clean build artifacts
clean:
    nix develop --command cargo clean

# Verify FUSE3 availability for zero-latency DnD
fuse-check:
    @echo "Checking FUSE3 availability..."
    nix develop --command pkg-config --modversion fuse3 && echo "✅ fuse3 OK — zero-latency DnD will be active" || echo "❌ fuse3 not found — DnD will use tmpfs fallback"
