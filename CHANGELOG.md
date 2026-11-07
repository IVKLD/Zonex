# Changelog

All notable changes to **Zonex** are documented in this file.

## [0.2.0] - 2026-11-07

### 🚀 Performance & Large Archive Architecture
- **$O(N)$ FUSE Tree Construction**: Replaced quadratic directory lookups with direct hash indexing `(parent_ino, name) -> dir_ino`, speeding up multi-gigabyte and multi-thousand file archive loads by several orders of magnitude.
- **Cached Navigation State**: Added `cached_visible` directory caching in `NavigationState`. Traversing folders, Back and Forward navigation now execute in $O(1)$ time without re-filtering or reparsing entries.
- **Debounced Search (150ms)**: Search bar now debounces rapid typing to eliminate UI lag and repeated allocation during fast search input. Clearing or pressing Enter triggers instant filtering.
- **Fast ZIP Raw Header Traversal**: Switched ZIP listing to `by_index_raw` to eliminate disk seeks to local file data headers during archive enumeration.
- **Linear Row Cleanup**: Reverse row disposal in file list views prevents $O(N^2)$ array shifts and eliminates GTK child disposal warnings.
- **Async Event Loop**: Completely removed busy-polling timers (`timeout_add_local` 60Hz), migrating the entire application to native `gio::spawn_blocking` and `glib::MainContext::spawn_local`.

### 🔒 Encryption & Security
- **User-Selectable Encryption Methods**:
  - **AES-256** (WinZip AES, strongest, recommended default)
  - **AES-192** (high security)
  - **AES-128** (standard security)
  - **ZipCrypto** (legacy PKWARE for retro compatibility)
- **Encryption Dialog UI**: Added algorithm dropdown selector with password confirmation and hide/peek controls.
- **Passphrase Detection**: Added support for detecting `passphrase` and `bad decrypt` error patterns in `libarchive`, properly triggering password prompts for encrypted non-ZIP archives.
- **Data Safety Confirmation**: Added confirmation message dialog before deleting files from archives, preventing catastrophic accidental deletions from keyboard presses.

### 🧭 Navigation & Modern User Experience
- **Interactive Breadcrumb Path Bar**: Replaced static text labels with clickable Adwaita breadcrumb buttons. Clicking any folder in the path navigates directly to it.
- **Shift+Click Range Selection**: Full multi-selection range selection when holding Shift, plus Ctrl+Click for item toggling and click-drag preservation.
- **Status Bar Metrics**: Real-time counter of total items, total archive size, selected items count, and selected size formatted cleanly (B, KB, MB, GB).
- **In-Archive New Folder Creation**: Added `win.new_folder` (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>N</kbd>, header and context menu) allowing direct folder creation within ZIP and TAR archives.
- **Subfolder Context on Add/Drop**: Dropping files or adding files via dialog while inside a subfolder now accurately places them into the current active directory instead of root.
- **MIME System App Launch**: Files open with system default applications with ~0ms latency via FUSE mounts (or async cached extraction fallback). Enter/double-click launches default apps; Space opens Quick Preview.
- **File Export**: Added «Export Selected...» (<kbd>Ctrl</kbd>+<kbd>E</kbd>) with native Save As dialog for single items and Folder Picker for batch exports. Fixed «Extract Here» to target the archive directory.
- **File Dialog Format Filters**: Configured native file filters for all supported archive formats in both Open Archive and Create New Archive dialogs.
- **Enhanced Hotkeys**:
  - <kbd>Ctrl</kbd> + <kbd>F</kbd>: Focus search bar
  - <kbd>Escape</kbd>: Clear search bar and refocus list, or clear selection
  - <kbd>Alt</kbd> + <kbd>↑</kbd>: Navigate to parent folder
  - <kbd>F5</kbd> / <kbd>Ctrl</kbd> + <kbd>R</kbd>: Reload/refresh current view
  - <kbd>Alt</kbd> + <kbd>Enter</kbd>: View entry properties
  - <kbd>Ctrl</kbd> + <kbd>N</kbd>: Create new archive
  - <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>N</kbd>: Create new folder in archive
- **FUSE Cleanup**: Automatic cleanup of stale orphan mount directories in runtime folder on startup.

---
