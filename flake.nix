{
  description = "Zonex - Wayland-native Archive Manager in Rust with Zero-Latency FUSE Streaming";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    let
      # NixOS Module for system-wide configuration
      nixosModule = { config, lib, pkgs, ... }: {
        options.programs.zonex = {
          enable = lib.mkEnableOption "Zonex Wayland Archive Manager";
        };
        config = lib.mkIf config.programs.zonex.enable {
          environment.systemPackages = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          programs.fuse.userAllowOther = true;
        };
      };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        desktopItem = pkgs.makeDesktopItem {
          name = "io.github.zonex.ArchiveManager";
          desktopName = "Zonex";
          comment = "Wayland-Native Archive Manager with Zero-Latency Drag-and-Drop";
          exec = "zonex %F";
          icon = "io.github.zonex.ArchiveManager";
          terminal = false;
          type = "Application";
          categories = [ "Utility" "Archiving" "Compression" "GTK" ];
          mimeTypes = [
            "application/zip"
            "application/x-tar"
            "application/x-compressed-tar"
            "application/x-bzip-compressed-tar"
            "application/x-xz-compressed-tar"
            "application/x-zstd-compressed-tar"
            "application/x-7z-compressed"
            "application/x-rar"
          ];
          keywords = [ "archive" "zip" "tar" "compress" "extract" "dnd" "wayland" ];
          startupWMClass = "io.github.zonex.ArchiveManager";
        };

        commonMeta = with pkgs.lib; {
          description = "Wayland-native Archive Manager in Rust with Zero-Latency FUSE Drag-and-Drop";
          homepage = "https://github.com/IVKLD/Zonex";
          license = licenses.mit;
          platforms = platforms.linux;
          mainProgram = "zonex";
        };

        zonexSrcPkg = pkgs.rustPlatform.buildRustPackage {
          pname = "zonex";
          version = "0.2.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            copyDesktopItems
            libarchive
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

          desktopItems = [ desktopItem ];

          postInstall = ''
            mkdir -p $out/share/icons/hicolor/scalable/apps
            cp data/icons/hicolor/scalable/apps/io.github.zonex.ArchiveManager.svg $out/share/icons/hicolor/scalable/apps/
          '';

          meta = commonMeta;
        };

        hasPrebuiltBin = builtins.pathExists (./bin + "/${system}/zonex");

        zonexBinPkg = pkgs.stdenv.mkDerivation {
          pname = "zonex";
          version = "0.2.0";
          src = ./.;

          nativeBuildInputs = with pkgs; [
            autoPatchelfHook
            wrapGAppsHook4
            copyDesktopItems
          ];

          buildInputs = with pkgs; [
            gtk4
            libadwaita
            libarchive
            glib
            cairo
            pango
            gdk-pixbuf
            fuse3
            gsettings-desktop-schemas
          ];

          installPhase = ''
            runHook preInstall
            mkdir -p $out/bin
            cp bin/${system}/zonex $out/bin/zonex
            chmod +x $out/bin/zonex

            mkdir -p $out/share/icons/hicolor/scalable/apps
            cp data/icons/hicolor/scalable/apps/io.github.zonex.ArchiveManager.svg $out/share/icons/hicolor/scalable/apps/
            runHook postInstall
          '';

          desktopItems = [ desktopItem ];
          meta = commonMeta;
        };

        # Default package uses prebuilt binary (instant install, NO compiling on user machine)
        defaultPkg = if hasPrebuiltBin then zonexBinPkg else zonexSrcPkg;
      in
      {
        packages.default = defaultPkg;
        packages.zonex = defaultPkg;
        packages.zonex-bin = zonexBinPkg;
        packages.zonex-source = zonexSrcPkg;

        apps.default = flake-utils.lib.mkApp {
          drv = defaultPkg;
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            clippy
            rustfmt
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
        };
      }
    ) // {
      nixosModules.default = nixosModule;
      nixosModules.zonex = nixosModule;
      overlays.default = final: prev: {
        zonex = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    };
}
