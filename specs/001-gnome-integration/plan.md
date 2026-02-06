# Implementation Plan: GNOME Desktop Integration

**Branch**: `001-gnome-integration` | **Date**: 2026-02-05 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-gnome-integration/spec.md`

## Summary

Implement the Phase 3 "GNOME Integration" of the LNXDrive project: a multi-component desktop integration layer that provides native GNOME experience for the LNXDrive cloud sync system. The integration consists of four deliverables — a Nautilus file manager extension (C), a GNOME Shell status indicator (GJS), a GTK4/libadwaita preferences panel with onboarding wizard (Rust), and a GNOME Online Accounts provider (C, P3). All components communicate with the existing `lnxdrive-daemon` exclusively via the D-Bus API (`org.enigmora.LNXDrive`).

## Technical Context

**Language/Version**: Rust 1.83+ (preferences/onboarding), C11 (Nautilus extension, GOA provider), GJS/ESM (Shell extension)
**Primary Dependencies**: gtk4-rs 0.9.x, libadwaita-rs 0.7-0.8.x, zbus 5.x, libnautilus-extension-4, GJS (GNOME Shell), oauth2 5.x, gettextrs 0.7 *(note: lnxdrive-ipc is a planned shared crate not yet published; D-Bus proxies are hand-rolled via zbus for now)*
**Storage**: N/A (all state managed by daemon; preferences backed by `~/.config/lnxdrive/config.yaml` via D-Bus)
**Testing**: cargo test (Rust), Meson test (C), manual + scripted (GJS), D-Bus mock daemon for all
**Target Platform**: Linux with GNOME 45, 46, 47 (Fedora 40+, Ubuntu 24.04+)
**Project Type**: Multi-component desktop integration (3 languages, unified Meson build)
**Performance Goals**: Overlay icons < 500ms for 5000+ files (FR-027/SC-005), indicator updates < 3s (SC-003)
**Constraints**: libnautilus-extension-4 only (FR-030), ESM modules only (GNOME 45+), i18n via gettext (FR-035)
**Scale/Scope**: ~5K LOC Rust, ~1K LOC C, ~500 LOC GJS, 7 emblem icons, 37 functional requirements

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The project constitution (`.specify/memory/constitution.md`) is an unfilled template — no active gates to enforce. The project's architectural governance comes from the Design Guide, which has been consulted and respected throughout:

- **Hexagonal architecture**: GNOME components are pure driving adapters consuming D-Bus ports — no core logic in UI. ✅
- **UI as implementation detail**: All components are interchangeable per the guide's Principle #4. ✅
- **D-Bus as communication channel**: All UI-to-daemon communication via `org.enigmora.LNXDrive`. ✅
- **YAML as source of truth**: Preferences panel reads/writes config through daemon, not directly. ✅
- **Conventional commits + branch strategy**: Working on `001-gnome-integration` branch. ✅

## Project Structure

### Documentation (this feature)

```text
specs/001-gnome-integration/
├── plan.md              # This file
├── spec.md              # Feature specification (37 FRs, 6 user stories)
├── research.md          # Phase 0: Technology research (6 decisions)
├── data-model.md        # Phase 1: Entity definitions and state transitions
├── quickstart.md        # Phase 1: Development setup guide
├── contracts/
│   └── dbus-gnome-contracts.md   # Phase 1: D-Bus interface contracts
├── checklists/
│   └── requirements.md           # Spec quality validation
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
lnxdrive-gnome/
├── meson.build                         # Top-level Meson build
├── meson_options.txt                   # Build options
│
├── nautilus-extension/                 # C shared library (liblnxdrive-nautilus.so)
│   ├── meson.build
│   ├── src/
│   │   ├── lnxdrive-extension.c       # Module entry points (initialize, list_types, shutdown)
│   │   ├── lnxdrive-info-provider.c   # InfoProvider: overlay icons via add_emblem()
│   │   ├── lnxdrive-menu-provider.c   # MenuProvider: context menu (pin, unpin, sync)
│   │   ├── lnxdrive-column-provider.c # ColumnProvider: status + last_sync columns
│   │   └── lnxdrive-dbus-client.c     # GDBus proxy: status cache + signal subscription
│   └── icons/                          # 7 SVG emblem icons (hicolor/scalable/emblems/)
│       ├── lnxdrive-synced.svg
│       ├── lnxdrive-cloud-only.svg
│       ├── lnxdrive-syncing.svg
│       ├── lnxdrive-pending.svg
│       ├── lnxdrive-conflict.svg
│       ├── lnxdrive-error.svg
│       └── lnxdrive-unknown.svg
│
├── shell-extension/                    # GJS GNOME Shell extension
│   └── lnxdrive-indicator@enigmora.com/
│       ├── extension.js                # PanelMenu.Button indicator
│       ├── metadata.json               # shell-version: ["45", "46", "47"]
│       ├── prefs.js                    # Minimal prefs (launch main prefs app)
│       ├── stylesheet.css              # Custom Shell CSS
│       ├── dbus.js                     # Gio.DBusProxy.makeProxyWrapper definitions
│       ├── indicator.js                # Indicator icon + state management
│       └── menuItems.js                # PopupMenu item builders (progress, conflicts, quota)
│
├── preferences/                        # Rust GTK4/libadwaita application
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                     # Entry point, gettext init, app launch
│   │   ├── app.rs                      # AdwApplication: lifecycle, onboarding detection
│   │   ├── window.rs                   # Main window (hosts prefs or onboarding)
│   │   ├── onboarding/                 # First-run wizard (FR-031 to FR-034)
│   │   │   ├── mod.rs
│   │   │   ├── auth_page.rs            # OAuth2 PKCE via system browser
│   │   │   ├── folder_page.rs          # Sync root folder selection
│   │   │   └── confirm_page.rs         # Summary + start sync
│   │   ├── preferences/                # Preferences dialog (FR-013 to FR-018)
│   │   │   ├── mod.rs
│   │   │   ├── account_page.rs         # Account info + quota
│   │   │   ├── sync_page.rs            # Sync options + selective sync tree
│   │   │   ├── advanced_page.rs        # Exclusions + bandwidth
│   │   │   └── folder_tree.rs          # ListView + TreeListModel + CheckButton
│   │   └── dbus_client.rs              # zbus proxy (hand-rolled; lnxdrive-ipc pending)
│   └── data/
│       ├── com.enigmora.LNXDrive.Preferences.desktop.in
│       ├── com.enigmora.LNXDrive.Preferences.metainfo.xml.in
│       └── com.enigmora.LNXDrive.Preferences.gschema.xml
│
├── po/                                 # i18n translations (gettext)
│   ├── POTFILES.in
│   ├── LINGUAS
│   └── lnxdrive-gnome.pot
│
├── data/                               # Shared data files
│   └── icons/hicolor/
│       ├── scalable/apps/com.enigmora.LNXDrive.svg
│       └── symbolic/apps/com.enigmora.LNXDrive-symbolic.svg
│
└── tests/                              # Integration tests
    ├── test-nautilus-extension.py       # Python script: D-Bus mock + verify overlay behavior
    ├── test-shell-extension.js          # GJS test: indicator lifecycle
    └── mock-dbus-daemon.py             # Mock D-Bus server for development/testing
```

**Structure Decision**: Multi-component layout following the guide's `lnxdrive-gnome` repository structure. Three distinct components (Nautilus extension in C, Shell extension in GJS, Preferences app in Rust) unified under a single Meson build system. This mirrors how other GNOME projects handle multi-language components (e.g., GNOME Settings has C backends + GJS/Python plugins).

## Implementation Phases

### Phase A: Foundation & D-Bus Mock (P1 prerequisite)

**Goal**: Establish build system and D-Bus mock so all components can be developed and tested independently of the real daemon.

1. **Meson build skeleton**: Top-level `meson.build` with subdir() for each component. Configure pkg-config deps (`libnautilus-extension-4`, `gtk4`, `libadwaita-1`).
2. **D-Bus mock daemon** (`tests/mock-dbus-daemon.py`): Python script using `pydbus` that implements all consumed D-Bus interfaces with fake data. Emits signals on a timer for testing real-time updates.
3. **Emblem icon set**: Design and install 7 SVG emblem icons following GNOME icon guidelines (16x16 symbolic, hicolor).
4. **i18n infrastructure**: `po/` directory, `POTFILES.in`, Meson `i18n.gettext()` configuration.

### Phase B: Nautilus Extension (P1 — US1 + US2)

**Goal**: Overlay icons and context menu in Nautilus.

1. **Module skeleton** (`lnxdrive-extension.c`): `nautilus_module_initialize()`, `list_types()`, `shutdown()`. Register GTypes implementing `InfoProvider`, `MenuProvider`, `ColumnProvider`.
2. **D-Bus client** (`lnxdrive-dbus-client.c`): `GDBusProxy` to `org.enigmora.LNXDrive.Files`. Local cache of file statuses (hash map: path → status). Subscribe to `FileStatusChanged` signal. Handle daemon unavailability (`notify::g-name-owner`).
3. **InfoProvider** (`lnxdrive-info-provider.c`): `update_file_info()` — check if file is under sync root, query status from cache (or batch-query via D-Bus), call `add_emblem()` with appropriate icon name, `add_string_attribute()` for column data. Return `NAUTILUS_OPERATION_COMPLETE` from cache or `IN_PROGRESS` for async.
4. **MenuProvider** (`lnxdrive-menu-provider.c`): `get_file_items()` — check file status, build "LNXDrive" submenu with state-appropriate actions (Pin, Unpin, Sync Now). Connect `activate` signals to D-Bus method calls.
5. **ColumnProvider** (`lnxdrive-column-provider.c`): Two columns — "LNXDrive Status" and "Last Synced".

### Phase C: GNOME Shell Extension (P2 — US3)

**Goal**: Status indicator in the top bar.

1. **Extension skeleton**: `metadata.json` (shell-version: ["45", "46", "47"]), `extension.js` with `enable()`/`disable()` lifecycle.
2. **D-Bus proxy** (`dbus.js`): `makeProxyWrapper()` for `.Sync`, `.Status`, `.Manager` interfaces. Async construction. `notify::g-name-owner` for daemon tracking.
3. **Indicator** (`indicator.js`): `PanelMenu.Button` subclass. Icon state management (idle/syncing/paused/error/offline). Icon swap on `SyncStateChanged` signal.
4. **Menu items** (`menuItems.js`): `PopupMenuItem` for sync progress (file + percentage), `PopupSeparatorMenuItem`, conflict count with click handler, quota bar (used/total), Pause/Resume toggle, Sync Now action, Preferences launcher.
5. **Stylesheet** (`stylesheet.css`): Custom styles for progress display, quota bar, status colors.

### Phase D: Preferences Panel (P2 — US4)

**Goal**: GTK4/libadwaita preferences application.

1. **Rust project setup**: `Cargo.toml` with gtk4, libadwaita, zbus, lnxdrive-ipc, gettextrs. `main.rs` with gettext init and `AdwApplication` launch.
2. **D-Bus client** (`dbus_client.rs`): Hand-rolled zbus proxies (lnxdrive-ipc integration deferred to cross-repo milestone). Spawn on `glib::MainContext`. Handle daemon connection/disconnection.
3. **Account page** (`account_page.rs`): `AdwPreferencesPage` showing account email, display name, quota (progress bar), sign-out button.
4. **Sync page** (`sync_page.rs`): `AdwSwitchRow` for auto-sync, `AdwComboRow` for conflict policy, `AdwSpinRow` for sync interval. Selective sync folder tree (see below).
5. **Folder tree** (`folder_tree.rs`): `gtk::ListView` + `TreeListModel` + `TreeExpander` + `CheckButton`. Lazy loading via `GetRemoteFolderTree()` D-Bus call. Parent/child checkbox propagation.
6. **Advanced page** (`advanced_page.rs`): Exclusion patterns list (add/edit/delete), bandwidth limit spin rows.
7. **GSettings schema**: For local UI preferences (window size, last page viewed). Sync config goes through D-Bus, not GSettings.

### Phase E: Onboarding Wizard (P1 — US5)

**Goal**: First-run setup experience.

1. **Detection logic** (`app.rs`): On startup, call `Auth.IsAuthenticated()` via D-Bus. If false, show onboarding wizard instead of preferences.
2. **Auth page** (`auth_page.rs`): Call `Auth.StartAuth()` to get OAuth2 URL. Launch system browser via `gio::AppInfo::launch_default_for_uri()`. Listen for `AuthStateChanged` signal. Show "Waiting for authentication..." with cancel button.
3. **Folder page** (`folder_page.rs`): Simple folder chooser (`gtk::FileDialog`) for sync root selection. Show suggested default (`~/OneDrive`).
4. **Confirm page** (`confirm_page.rs`): Summary of account + folder. "Start Syncing" button calls `SetConfig()` + `Sync.SyncNow()`. Redirect to Nautilus or show indicator.
5. **Cancel handling**: No partial config saved. On cancel, return to auth page.

### Phase F: GNOME Online Accounts Provider (P3 — US6)

**Goal**: Native account integration in GNOME Settings.

1. **GOA provider** (C): Register Microsoft account type in GNOME Online Accounts.
2. **OAuth2 with WebKitGTK**: Embedded web view for the auth flow.
3. **Token sharing**: Pass tokens to daemon via D-Bus `Auth.CompleteAuth()`.
4. **Account lifecycle**: Monitor GOA for account removal, notify daemon.

*Note: This phase is deferred (P3). The onboarding wizard (Phase E) provides a fully functional independent authentication path.*

## Key Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Nautilus extension language | C | Only language compatible with libnautilus-extension-4 |
| Shell extension language | GJS (ESM) | Only supported language for GNOME Shell extensions |
| Preferences app language | Rust (gtk4-rs + libadwaita-rs) | Guide prescription, excellent bindings, type safety |
| Nautilus API | libnautilus-extension-4 only | GNOME 45+ baseline, no legacy support (FR-030) |
| Shell indicator type | PanelMenu.Button | Full menu control needed for rich status display |
| D-Bus in Nautilus ext | GDBus (libgio) | Native GLib integration, GLib main loop compatible |
| D-Bus in Shell ext | Gio.DBusProxy.makeProxyWrapper | Recommended GJS D-Bus API with signal support |
| D-Bus in Prefs app | zbus v5 (via lnxdrive-ipc) | Pure Rust, glib MainContext integration |
| OAuth2 flow | System browser + loopback | RFC 8252 compliant, most secure |
| Folder tree widget | ListView + TreeListModel | GtkTreeView deprecated since GTK 4.10 |
| i18n | gettext (gettextrs + xtr) | GNOME standard, Meson integration |
| Build system | Meson (unified) | GNOME standard, handles C + GJS + Rust + i18n |

## Complexity Tracking

| Decision | Why This Complexity | Simpler Alternative Rejected Because |
|----------|---------------------|-------------------------------------|
| 3 languages (C + GJS + Rust) | Each component requires its platform-native language | Single-language not possible: Nautilus needs C, Shell needs GJS, guide prescribes Rust for GTK4 |
| Meson wrapping Cargo | Unified build/install for all components | Separate build systems would complicate packaging and installation |
| Local status cache in Nautilus ext | Performance: `update_file_info()` called per-file, cannot afford D-Bus round-trip each time | Direct D-Bus calls per file would degrade Nautilus with 5000+ files (FR-027) |
