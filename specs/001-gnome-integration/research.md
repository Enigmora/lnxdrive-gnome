# Research: GNOME Desktop Integration

**Branch**: `001-gnome-integration` | **Date**: 2026-02-05

---

## R1: Nautilus Extension API (libnautilus-extension-4)

### Decision
Use **libnautilus-extension-4** (API version 4.1) written in **C**, implementing three interfaces: `NautilusInfoProvider` (overlay icons), `NautilusMenuProvider` (context menu), and `NautilusColumnProvider` (custom columns).

### Rationale
- libnautilus-extension-4 is a deliberately minimal, model-based API that removed all direct GTK widget access.
- Overlay icons work via `NautilusFileInfo.add_emblem()` — emblems are the standard mechanism (used by Dropbox, Seafile, Insync).
- Custom emblem icons (`lnxdrive-synced`, `lnxdrive-syncing`, etc.) must be installed into the hicolor icon theme.
- The Rust bindings (`nautilus-extension-rs` v0.8.0) only target Nautilus 3 + GTK3 — **not compatible** with v4.
- C is the only language with guaranteed compatibility. The extension is a thin shared library that delegates to the daemon via D-Bus, so C complexity is minimal.
- Python via `nautilus-python` is a fallback but adds runtime dependency and is slower for `update_file_info` on large directories.

### Alternatives Considered
- **Rust with manual FFI**: Would require writing custom unsafe bindings from scratch. Not worth it for a thin IPC bridge.
- **Python (nautilus-python)**: Slower for per-file operations, adds python3 runtime dependency.
- **Vala**: GObject introspection annotations incomplete (GitLab issue #565).

### Key API Surfaces

| Interface | Purpose | Key Methods |
|-----------|---------|-------------|
| `NautilusInfoProvider` | Overlay icons + metadata | `update_file_info()`, `cancel_update()` |
| `NautilusMenuProvider` | Context menu items | `get_file_items()`, `get_background_items()` |
| `NautilusColumnProvider` | Custom list columns | `get_columns()` |
| `NautilusFileInfo` | File data access | `add_emblem()`, `add_string_attribute()`, `get_uri()`, `invalidate_extension_info()` |

### Communication with Daemon
- D-Bus via GDBus (`Gio.DBusProxy`) from C code.
- Cache file statuses locally; subscribe to `FileStatusChanged` signal for push updates.
- Call `invalidate_extension_info()` on affected files when status changes.

### Build System
- **Meson** producing a `shared_module` installed to `${libdir}/nautilus/extensions-4/`.
- pkg-config: `libnautilus-extension-4` provides `extensiondir`.
- Emblem icons installed to `${datadir}/icons/hicolor/scalable/emblems/`.

---

## R2: GNOME Shell Extension (GJS, GNOME 45-47)

### Decision
Write the Shell extension in **GJS** (JavaScript) using **ESM modules** (mandatory since GNOME 45), with a **`PanelMenu.Button`** indicator and **`Gio.DBusProxy.makeProxyWrapper()`** for D-Bus communication.

### Rationale
- GNOME 45 replaced the `imports` system with standard ECMAScript Modules (ESM). Extensions for GNOME 44- are incompatible with 45+.
- `PanelMenu.Button` gives full control over a rich dropdown menu (progress bars, multi-section status, quick actions). `QuickSettings.SystemIndicator` is better suited for simple toggles.
- `makeProxyWrapper()` is the recommended high-level D-Bus API: automatic signal marshalling, property caching, and `notify::g-name-owner` for daemon availability tracking.

### Multi-Version Strategy (45, 46, 47)
Write to GNOME 45 baseline, avoid deprecated APIs from the start:
- Use `add_child()` / `remove_child()` (not `add_actor()` — removed in 46)
- Avoid `Clutter.Color` (removed in 47, use `Cogl.Color()`)
- Version-gate only when absolutely necessary via `Config.PACKAGE_VERSION`

### Breaking Changes Across Versions

| GNOME Version | Relevant Change | Migration |
|---------------|-----------------|-----------|
| 45 (baseline) | ESM modules, `Extension` class | Use ESM, no backward compat |
| 46 | `Clutter.Container` removed | Use `add_child()` from start |
| 47 | `Clutter.Color` removed | Use `Cogl.Color()` if needed |

### Extension Structure
```
lnxdrive-indicator@enigmora.com/
├── extension.js          # Main entry point (PanelMenu.Button)
├── metadata.json         # shell-version: ["45", "46", "47"]
├── prefs.js              # GTK4 preferences (links to main prefs app)
├── stylesheet.css         # Custom Shell CSS
├── dbus.js               # D-Bus proxy definitions
├── indicator.js           # Indicator UI builder
├── menuItems.js           # PopupMenu item constructors
├── locale/               # Translations
└── schemas/              # GSettings
```

### Lifecycle Rules
- Create resources only in `enable()`, destroy only in `disable()`.
- Never import `Gdk`, `Gtk`, or `Adw` in `extension.js`.
- Disconnect all signals (both D-Bus and GObject) in `disable()`.
- `makeProxyWrapper()` must be called inside `enable()`, not at module scope.

---

## R3: GTK4 + libadwaita Preferences Panel (Rust)

### Decision
Build the preferences panel and onboarding wizard in **Rust** using **gtk4-rs** (v0.9.x) + **libadwaita-rs** (v0.7-0.8.x). Use `AdwPreferencesDialog` as the main container with `AdwPreferencesPage` / `AdwPreferencesGroup` / row widgets.

### Rationale
- The guide's repository structure doc explicitly prescribes Rust for `lnxdrive-gnome` with "gtk4-rs, libadwaita-rs — excellent bindings".
- gtk4-rs and libadwaita-rs are production-ready (MSRV: Rust 1.83+).
- `AdwPreferencesDialog` (replaces deprecated `AdwPreferencesWindow` since libadwaita 1.5) provides built-in search, navigation, and GNOME HIG compliance.

### Preferences Panel Structure
```
PreferencesDialog
├── Page: "Account" (cloud-symbolic)
│   ├── Group: "OneDrive Account" — account email, quota bar
│   └── Group: "Authentication" — Sign In / Sign Out
├── Page: "Sync" (folder-symbolic)
│   ├── Group: "Sync Options" — SwitchRow, ComboRow, SpinRow
│   └── Group: "Selective Sync" — TreeListModel folder tree
└── Page: "Advanced" (preferences-other-symbolic)
    ├── Group: "Exclusions" — pattern editor
    └── Group: "Network" — bandwidth limits
```

### Folder Tree with Checkboxes
- `GtkTreeView` is deprecated since GTK 4.10.
- Use `gtk::ListView` + `gtk::TreeListModel` + `gtk::TreeExpander` + `gtk::CheckButton` inside a `SignalListItemFactory`.
- Lazy loading: children populated only when node is expanded.
- Parent/child selection propagation implemented in signal handlers.

### OAuth2 PKCE Authentication
- Use `oauth2` crate (oauth2-rs) for PKCE flow logic.
- **Primary**: System browser + loopback redirect (RFC 8252 compliant, most secure).
- **Fallback**: Embedded `webkit6` WebView (for environments without system browser).
- Microsoft Graph scopes: `Files.ReadWrite.All`, `offline_access`.

### D-Bus via zbus
- Use `zbus` v5 with `#[proxy]` macros.
- Integrate with GTK4 by spawning zbus futures on `glib::MainContext` (not tokio).
- Reuse `lnxdrive-ipc` crate which already defines D-Bus proxy types.

### i18n via gettext
- `gettextrs` crate for runtime translation.
- `xtr` tool for extracting translatable strings from Rust source.
- Meson `i18n` module for building `.mo` files.
- Gettext domain: `lnxdrive-gnome`.

---

## R4: GNOME Online Accounts Provider (P3)

### Decision
Implement a GOA provider in **C** that registers a Microsoft account type in GNOME Settings → Online Accounts. Deferred to P3 priority.

### Rationale
- GOA providers are C-based GObject implementations loaded as shared libraries by gnome-online-accounts.
- The onboarding wizard (P1) provides an independent authentication path, making GOA non-blocking.
- GOA integration adds SSO polish (reuse existing Microsoft accounts) but requires significant platform-specific plumbing.

---

## R5: Technology Stack Reconciliation

### Decision
The guide contains a discrepancy: `01-stack-tecnologico.md` lists `lnxdrive-gnome` as "Python + GTK4", while `01-estructura-repositorios.md` lists it as "Rust (gtk4-rs, libadwaita-rs)". **Follow the repository structure doc** (more detailed, more recent).

### Final Component Languages

| Component | Language | Reason |
|-----------|----------|--------|
| Nautilus extension | C | Only lang with libnautilus-extension-4 support |
| GNOME Shell extension | GJS (JavaScript) | Only supported language for Shell extensions |
| Preferences panel + Onboarding | Rust (gtk4-rs + libadwaita-rs) | Guide prescription, excellent bindings |
| GOA provider (P3) | C | GOA provider interface is C-based |
| D-Bus client | Rust (zbus via lnxdrive-ipc) | Shared IPC crate |

---

## R6: Build System and Project Structure

### Decision
Use **Meson** as the unified build system for all components (C shared libraries, GJS extension, Rust application). Meson is the GNOME standard and handles i18n, icon installation, desktop files, and GSettings schema compilation.

### Rationale
- Nautilus extension requires Meson for `shared_module` + pkg-config integration.
- GNOME Shell extensions use a simple file copy but benefit from Meson for schema compilation and i18n.
- Rust binary can be built via `cargo` invoked from Meson (using `custom_target` or `find_program('cargo')`).
- Unified install targets for all components.

### Alternatives Considered
- **Cargo-only**: Cannot handle C shared libraries, icon installation, schema compilation, or desktop file installation.
- **CMake**: Not idiomatic for GNOME projects.
- **Separate build systems per component**: Increases packaging complexity.
