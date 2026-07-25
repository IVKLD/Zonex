{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    rustc
    pkg-config
    just
  ];

  buildInputs = with pkgs; [
    gtk4
    libadwaita
    libarchive
    glib
    cairo
    pango
    gdk-pixbuf
    gsettings-desktop-schemas
    fuse3
  ];

  shellHook = ''
    export GDK_BACKEND=wayland
    export RUST_BACKTRACE=1
    export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk4}/share/gsettings-schemas/${pkgs.gtk4.name}:$XDG_DATA_DIRS"
    export PKG_CONFIG_PATH="${pkgs.fuse3.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
  '';
}
