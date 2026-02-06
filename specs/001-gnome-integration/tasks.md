# Tasks: GNOME Desktop Integration

**Input**: Design documents from `/specs/001-gnome-integration/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md, pending-issues.md

**Organization**: Tasks grouped into **Stages** (project convention). Stages 3–6 target different components/languages and can execute in **parallel via subagents**.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel with other [P] tasks in the same stage (different files, no shared state)
- **[Story]**: User story traceability (US1–US6)
- All file paths relative to repository root

---

## Stage 1: Setup (Project Skeleton)

**Purpose**: Initialize Meson build system and project structure for all three components.

- [x] T001 Create top-level `meson.build` with project('lnxdrive-gnome', 'c', version: '0.1.0', license: 'GPL-3.0-or-later', meson_version: '>=0.62.0'), import i18n module, subdir() calls for nautilus-extension/, shell-extension/, preferences/, po/, data/
- [x] T002 [P] Create `meson_options.txt` with boolean options: `enable_nautilus` (default: true), `enable_shell` (default: true), `enable_preferences` (default: true), `enable_goa` (default: false)
- [x] T003 [P] Create `nautilus-extension/meson.build` declaring `shared_module('lnxdrive-nautilus')` with `libnautilus-extension-4` and `gio-2.0` pkg-config dependencies, source file list (placeholder), install_dir from `nautilus_ext.get_variable(pkgconfig: 'extensiondir')`
- [x] T004 [P] Create `shell-extension/meson.build` with install rules copying all files from `lnxdrive-indicator@enigmora.com/` to `datadir / 'gnome-shell' / 'extensions' / 'lnxdrive-indicator@enigmora.com'`
- [x] T005 [P] Create `preferences/Cargo.toml` with package metadata (name: "lnxdrive-preferences", version: "0.1.0", edition: "2021") and dependencies: gtk4 = { version = "0.9", features = ["v4_14"] }, libadwaita = { version = "0.7", features = ["v1_6"] }, zbus = "5", lnxdrive-ipc = { git = "https://github.com/enigmora/lnxdrive.git", package = "lnxdrive-ipc" }, oauth2 = "5", gettextrs = "0.7", serde = { version = "1", features = ["derive"] }, tokio = { version = "1", features = ["rt"] }
- [x] T006 [P] Create `preferences/meson.build` with custom_target wrapping `cargo build --release` in `preferences/`, install resulting binary to bindir as `lnxdrive-preferences`
- [x] T007 [P] Update `.gitignore` adding: `builddir/`, `preferences/target/`, `*.o`, `*.so`, `*.mo`, `*.pyc`, `__pycache__/`

**Checkpoint**: `meson setup builddir` succeeds (build files generated, no compilation yet)

---

## Stage 2: Foundation (D-Bus Mock, Icons, i18n, Data Files)

**Purpose**: Core infrastructure that MUST complete before ANY user story implementation. Provides the D-Bus mock for development/testing, emblem icons, app icons, i18n setup, and desktop integration files.

**⚠️ CRITICAL**: No user story work can begin until this stage is complete.

### D-Bus Mock Daemon

- [x] T008 Create `tests/mock-dbus-daemon.py` — Python script using `dbus-next` (or `pydbus`): acquire bus name `org.enigmora.LNXDrive` on session bus, register object at `/org/enigmora/LNXDrive`. Implement all 6 D-Bus interfaces per contracts/dbus-gnome-contracts.md: `.Files` (GetFileStatus returns from hardcoded dict, GetBatchFileStatus iterates dict, PinFile/UnpinFile/SyncPath log + emit FileStatusChanged, GetConflicts returns sample paths), `.Sync` (SyncNow/Pause/Resume toggle state, SyncStatus/PendingChanges/LastSyncTime properties, emit SyncStarted/SyncCompleted/SyncProgress/ConflictDetected on timer), `.Status` (GetQuota returns 5GB/15GB, GetAccountInfo returns sample dict, ConnectionStatus property, emit QuotaChanged/ConnectionChanged periodically), `.Manager` (Start/Stop/Restart log, GetStatus returns "running", Version="0.1.0-mock"/IsRunning=true properties), `.Settings` (GetConfig returns sample YAML, SetConfig validates+logs, GetSelectedFolders/SetSelectedFolders with sample paths, GetExclusionPatterns/SetExclusionPatterns with ["*.tmp","*.bak"], GetRemoteFolderTree returns JSON tree `{"name":"root","path":"/","children":[{"name":"Documents","path":"/Documents","children":[]},{"name":"Photos","path":"/Photos","children":[]}]}`, emit ConfigChanged on Set*), `.Auth` (StartAuth returns mock URL + state, CompleteAuth returns true, IsAuthenticated returns configurable --authenticated flag, Logout resets, emit AuthStateChanged). Parse CLI: `--authenticated` (default false), `--signal-interval=5` (seconds), `--sync-root=/home/user/OneDrive`. Graceful shutdown on SIGINT.

### Emblem Icons (SVG, GNOME symbolic style: 16×16 viewBox, single color `currentColor`)

- [x] T009 [P] Create `nautilus-extension/icons/lnxdrive-synced.svg` — green checkmark emblem, 16×16 viewBox, stroke currentColor, simple check path ✓
- [x] T010 [P] Create `nautilus-extension/icons/lnxdrive-cloud-only.svg` — cloud outline emblem, 16×16 viewBox, stroke currentColor, cloud silhouette path
- [x] T011 [P] Create `nautilus-extension/icons/lnxdrive-syncing.svg` — two circular arrows emblem, 16×16 viewBox, stroke currentColor, sync rotation motif
- [x] T012 [P] Create `nautilus-extension/icons/lnxdrive-pending.svg` — clock/hourglass emblem, 16×16 viewBox, stroke currentColor, clock face with hands
- [x] T013 [P] Create `nautilus-extension/icons/lnxdrive-conflict.svg` — warning triangle with exclamation, 16×16 viewBox, stroke currentColor
- [x] T014 [P] Create `nautilus-extension/icons/lnxdrive-error.svg` — circle with X/cross, 16×16 viewBox, stroke currentColor
- [x] T015 [P] Create `nautilus-extension/icons/lnxdrive-unknown.svg` — circle with question mark, 16×16 viewBox, stroke currentColor (FR-001: estado "desconocido" when daemon unavailable)

### Application Icons

- [x] T016 [P] Create `data/icons/hicolor/scalable/apps/com.enigmora.LNXDrive.svg` — full-color app icon: cloud with sync arrows motif, Enigmora brand colors, 128×128 viewBox following GNOME app icon guidelines
- [x] T017 [P] Create `data/icons/hicolor/symbolic/apps/com.enigmora.LNXDrive-symbolic.svg` — monochrome symbolic icon for Shell panel, 16×16 viewBox, single color currentColor, simplified cloud motif

### Icon Installation (Meson)

- [x] T018 Add emblem icon install rules to `nautilus-extension/meson.build`: `install_data()` for each SVG in `nautilus-extension/icons/` to `get_option('datadir') / 'icons' / 'hicolor' / 'scalable' / 'emblems'`. Add comment: "SVG emblems scale natively for HiDPI (FR-029) — ref pending issue G2"
- [x] T019 [P] Create `data/meson.build` with install rules: scalable app icon to `datadir / 'icons' / 'hicolor' / 'scalable' / 'apps'`, symbolic app icon to `datadir / 'icons' / 'hicolor' / 'symbolic' / 'apps'`, add `gnome.post_install(gtk_update_icon_cache: true)` in top-level meson.build

### i18n Infrastructure

- [x] T020 Create `po/POTFILES.in` listing all translatable source files: `preferences/src/main.rs`, `preferences/src/app.rs`, `preferences/src/window.rs`, `preferences/src/onboarding/auth_page.rs`, `preferences/src/onboarding/folder_page.rs`, `preferences/src/onboarding/confirm_page.rs`, `preferences/src/preferences/account_page.rs`, `preferences/src/preferences/sync_page.rs`, `preferences/src/preferences/advanced_page.rs`, `preferences/src/preferences/folder_tree.rs`, `preferences/data/com.enigmora.LNXDrive.Preferences.desktop.in`, `preferences/data/com.enigmora.LNXDrive.Preferences.metainfo.xml.in`
- [x] T021 [P] Create `po/LINGUAS` with initial language: `en`
- [x] T022 [P] Create `po/lnxdrive-gnome.pot` — initial empty gettext template with standard header (Project-Id-Version, POT-Creation-Date, charset=UTF-8)
- [x] T023 [P] Create `po/meson.build` with `i18n.gettext('lnxdrive-gnome', preset: 'glib')`. Add `subdir('po')` to top-level `meson.build`.

### Desktop Integration Files

- [x] T024 Create `preferences/data/com.enigmora.LNXDrive.Preferences.desktop.in` — Type=Application, Name=LNXDrive Preferences (translatable), GenericName=Cloud Sync Settings (translatable), Comment=Configure LNXDrive cloud synchronization (translatable), Exec=lnxdrive-preferences, Icon=com.enigmora.LNXDrive, Categories=Settings;GTK;, StartupNotify=true, Terminal=false
- [x] T025 [P] Create `preferences/data/com.enigmora.LNXDrive.Preferences.metainfo.xml.in` — AppStream metadata: component type=desktop-application, id=com.enigmora.LNXDrive.Preferences, translatable name/summary/description, project_license=GPL-3.0-or-later, url type=homepage, screenshots placeholder, content_rating type=oars-1.1, releases section with initial 0.1.0
- [x] T026 [P] Create `preferences/data/com.enigmora.LNXDrive.Preferences.gschema.xml` — schema id="com.enigmora.LNXDrive.Preferences" path="/com/enigmora/LNXDrive/Preferences/", keys: window-width (int, default 800), window-height (int, default 600), last-page (string, default "account"). Ref pending issue G3: these are local UI prefs only, not sync config.
- [x] T027 [P] Add desktop file/metainfo/gschema install rules to `preferences/meson.build`: `i18n.merge_file()` for .desktop.in, `install_data()` for metainfo, `gnome.compile_schemas()` for gschema

**Checkpoint**: `meson compile -C builddir` succeeds for C stubs. `python3 tests/mock-dbus-daemon.py --help` runs. All SVGs valid. `meson install -C builddir --destdir=/tmp/test-install` installs icons, desktop files, schemas correctly.

---

## Stage 3: US1 + US2 — Nautilus Extension (Priority: P1) 🎯 MVP

**Goal**: Overlay icons showing sync status on every file in the LNXDrive folder (US1) + context menu with Pin/Unpin/Sync actions (US2).

**Independent Test**: Start mock D-Bus daemon → `meson install -C builddir` → restart Nautilus → navigate to mock sync root → overlay icons appear per file status → right-click → "LNXDrive" submenu shows correct actions → execute action → overlay updates.

**Language**: C11 | **Directory**: `nautilus-extension/src/`

**Pending Issues**: I2 (Excluded icon decision), I3 (Unknown vs Excluded asymmetry), FR-008 (visual feedback)

### D-Bus Client (shared by all providers)

- [x] T028 [US1] Create `nautilus-extension/src/lnxdrive-dbus-client.h` — declare `LnxdriveDbusClient` GObject type. Public API: `LnxdriveDbusClient *lnxdrive_dbus_client_get_default(void)` (singleton), `const char *lnxdrive_dbus_client_get_file_status(self, const char *path)`, `GHashTable *lnxdrive_dbus_client_get_batch_file_status(self, const char **paths, gsize n_paths)`, `void lnxdrive_dbus_client_pin_file(self, const char *path, GAsyncReadyCallback cb, gpointer data)`, `void lnxdrive_dbus_client_unpin_file(self, ...)`, `void lnxdrive_dbus_client_sync_path(self, ...)`, `gboolean lnxdrive_dbus_client_is_daemon_running(self)`, `const char *lnxdrive_dbus_client_get_sync_root(self)`. Define signal `file-status-changed(path, status)`. Define typedef `LnxdriveInvalidateFunc` callback.
- [x] T029 [US1] Create `nautilus-extension/src/lnxdrive-dbus-client.c` — implement: `G_DEFINE_TYPE(LnxdriveDbusClient, ...)`. Private struct: `GDBusProxy *files_proxy`, `GHashTable *status_cache` (string→string), `char *sync_root`, `gboolean daemon_running`, `LnxdriveInvalidateFunc invalidate_cb`. `_init()`: create `GDBusProxy` for `org.enigmora.LNXDrive` at `/org/enigmora/LNXDrive` interface `.Files` on session bus async. `_on_proxy_ready()`: subscribe to `FileStatusChanged` signal → update cache + emit `file-status-changed` glib signal + call invalidate_cb. Subscribe `notify::g-name-owner`: on NULL → set `daemon_running=false`, set all cache values to "unknown" (FR-025, SC-008); on non-NULL → set `daemon_running=true`, re-query batch status. `_get_file_status()`: lookup in cache, return "unknown" if not found or daemon not running. `_get_batch_file_status()`: sync D-Bus call to `GetBatchFileStatus`, populate cache from result, return result. `_pin_file()` / `_unpin_file()` / `_sync_path()`: async D-Bus calls with GAsyncResult pattern, propagate errors (InsufficientDiskSpace, FileInUse, InvalidPath). Add comment block with terminology glossary: "CloudOnly = cloud-only (D-Bus) = placeholder (user-facing); UnpinFile = unpin + dehydrate" (ref pending issues T1, T2).

### Module Entry Point

- [x] T030 [US1] Create `nautilus-extension/src/lnxdrive-extension.c` — implement `nautilus_module_initialize(GTypeModule *module)`: register `LnxdriveInfoProvider`, `LnxdriveMenuProvider`, `LnxdriveColumnProvider` GTypes with the GTypeModule. Implement `nautilus_module_list_types(const GType **types, int *n_types)`: return array of 3 GTypes. Implement `nautilus_module_shutdown()`: release D-Bus client singleton. Include `<nautilus-extension.h>`.

### Info Provider — Overlay Icons (US1)

- [x] T031 [US1] Create `nautilus-extension/src/lnxdrive-info-provider.h` — declare `LnxdriveInfoProvider` type implementing `NautilusInfoProvider` interface
- [x] T032 [US1] Create `nautilus-extension/src/lnxdrive-info-provider.c` — `G_DEFINE_TYPE_WITH_CODE` implementing `NautilusInfoProviderInterface`. Implement `update_file_info(NautilusInfoProvider *provider, NautilusFileInfo *file, GClosure *update_complete, NautilusOperationHandle **handle)`: (1) get URI via `nautilus_file_info_get_uri(file)`, convert to local path, (2) check if path starts with sync_root from D-Bus client — if not return `NAUTILUS_OPERATION_COMPLETE` with no emblem, (3) query status from D-Bus client cache via `_get_file_status()`, (4) map status string to emblem: "synced"→"lnxdrive-synced", "cloud-only"→"lnxdrive-cloud-only", "syncing"→"lnxdrive-syncing", "pending"→"lnxdrive-pending", "conflict"→"lnxdrive-conflict", "error"→"lnxdrive-error", "unknown"→"lnxdrive-unknown", "excluded"→no emblem (pending issue I2: excluded files show no emblem by design, add code comment), (5) call `nautilus_file_info_add_emblem(file, emblem_name)`, (6) call `nautilus_file_info_add_string_attribute(file, "LNXDrive::status", status)` and `nautilus_file_info_add_string_attribute(file, "LNXDrive::last_sync", formatted_time)`, (7) if status was from cache return `NAUTILUS_OPERATION_COMPLETE`, else if cache miss start async batch query and return `NAUTILUS_OPERATION_IN_PROGRESS`. Register D-Bus client `file-status-changed` signal to call `nautilus_file_info_invalidate_extension_info()` on affected files.
- [x] T033 [US1] Implement `cancel_update(NautilusInfoProvider *provider, NautilusOperationHandle *handle)` in `lnxdrive-info-provider.c` — cancel any pending async D-Bus query associated with this handle, free resources

### Column Provider — Custom Columns (US1)

- [x] T034 [P] [US1] Create `nautilus-extension/src/lnxdrive-column-provider.h` — declare `LnxdriveColumnProvider` implementing `NautilusColumnProvider` interface
- [x] T035 [P] [US1] Create `nautilus-extension/src/lnxdrive-column-provider.c` — `G_DEFINE_TYPE_WITH_CODE` implementing `NautilusColumnProviderInterface`. Implement `get_columns()`: return `GList*` with 2 `NautilusColumn` objects: (1) name="LNXDrive::status", attribute="LNXDrive::status", label="LNXDrive Status", description="Sync status of the file" and (2) name="LNXDrive::last_sync", attribute="LNXDrive::last_sync", label="Last Synced", description="Time of last successful sync". Both columns populated by InfoProvider via `add_string_attribute()`.

### Menu Provider — Context Menu (US2)

- [x] T036 [US2] Create `nautilus-extension/src/lnxdrive-menu-provider.h` — declare `LnxdriveMenuProvider` implementing `NautilusMenuProvider` interface
- [x] T037 [US2] Create `nautilus-extension/src/lnxdrive-menu-provider.c` — `G_DEFINE_TYPE_WITH_CODE` implementing `NautilusMenuProviderInterface`. Implement `get_file_items(NautilusMenuProvider *provider, GList *files)`: (1) check if daemon running via D-Bus client — if not, return single disabled `NautilusMenuItem` "LNXDrive — Service not running", (2) iterate files, check if ANY is under sync_root — if none return NULL (FR-005), (3) collect aggregate status: has_cloud_only, has_pinned, has_any_syncable from selection, (4) create parent `NautilusMenuItem` "LNXDrive" as submenu container, (5) if has_cloud_only: add "Keep Available Offline" item → connect `activate` to callback iterating cloud-only files calling `_pin_file()` (FR-006), (6) if has_pinned: add "Free Up Space" item → connect `activate` to callback iterating pinned files calling `_unpin_file()` (FR-006), (7) always add "Sync Now" item → connect `activate` to callback iterating all files calling `_sync_path()` (FR-006), (8) handle multi-selection: iterate all files in `GList *files` for each action (FR-007). Return `GList*` with the parent item.
- [x] T038 [US2] Implement `get_background_items(NautilusMenuProvider *provider, NautilusFileInfo *current_folder)` in `lnxdrive-menu-provider.c` — check if current_folder is sync_root or subdirectory. If yes: return GList with "LNXDrive → Sync This Folder" item, connect `activate` to `_sync_path()` with folder path.

### Error Handling (FR-036, FR-037)

- [x] T039 [US2] Implement D-Bus error handling in menu action callbacks in `lnxdrive-menu-provider.c`: in `_pin_file` async callback: catch `org.enigmora.LNXDrive.Error.InsufficientDiskSpace` → send `GNotification` via `g_application_send_notification()` with title "Not Enough Disk Space" and body "Cannot download file — insufficient disk space available" (FR-036). In `_unpin_file` async callback: catch `org.enigmora.LNXDrive.Error.FileInUse` → send `GNotification` "File In Use" / "Cannot free space — file is being used by another application" (FR-037). Catch `InvalidPath` → "File not in sync folder". Catch generic `GError` → "LNXDrive: Operation failed" with error message. Ref pending issue FR-008: GNotification provides explicit visual feedback beyond overlay icon change.

### Build Finalization

- [x] T040 [US1] Update `nautilus-extension/meson.build`: set source files list to [lnxdrive-extension.c, lnxdrive-dbus-client.c, lnxdrive-info-provider.c, lnxdrive-menu-provider.c, lnxdrive-column-provider.c], add c_args with `-DGETTEXT_PACKAGE="lnxdrive-gnome"`, verify shared_module compiles and installs to nautilus extensiondir

**Checkpoint**: `meson compile -C builddir` produces `liblnxdrive-nautilus.so`. With mock daemon running (`python3 tests/mock-dbus-daemon.py --authenticated`), install extension, restart Nautilus (`nautilus -q && nautilus &`), navigate to sync root → overlay icons appear → right-click shows LNXDrive submenu → actions work. **FR-001, FR-002, FR-003, FR-004, FR-005, FR-006, FR-007, FR-008, FR-025, FR-026, FR-027, FR-029, FR-030, FR-036, FR-037 covered.**

---

## Stage 4: US5 — Onboarding Wizard (Priority: P1)

**Goal**: First-run setup wizard: OAuth2 authentication → folder selection → start sync.

**Independent Test**: With mock daemon (`--authenticated=false`) → `cargo run` in preferences/ → wizard appears → auth step opens browser with mock URL → mock daemon emits AuthStateChanged → wizard advances → select folder → confirm → sync starts. Cancel → restart shows wizard again.

**Language**: Rust (gtk4-rs + libadwaita-rs) | **Directory**: `preferences/src/`

**Pending Issues**: U3 (OAuth2 loopback owner), U4 (CompleteAuth purpose)

### Rust App Scaffold (shared with Stage 6)

- [x] T041 [US5] Create `preferences/src/main.rs` — call `gettextrs::setlocale(LocaleCategory::LcAll, "")`, `gettextrs::bindtextdomain("lnxdrive-gnome", env!("LOCALEDIR")).unwrap()`, `gettextrs::textdomain("lnxdrive-gnome").unwrap()`. Create `LnxdriveApp` instance, call `app.run()`, return `std::process::ExitCode` from result.
- [x] T042 [US5] Create `preferences/src/app.rs` — define `LnxdriveApp` as `adw::Application` subclass via `glib::wrapper!` + `glib::Object` subclass macro. Application ID: `"com.enigmora.LNXDrive.Preferences"`. Implement `startup` signal handler: load GSettings schema, register GActions. Implement `activate` signal handler: create `LnxdriveWindow`, check `dbus_client.is_authenticated().await` → if false call `window.show_onboarding()`, if true call `window.show_preferences()` (FR-031). Handle D-Bus connection failure: show error `AdwStatusPage` "Cannot connect to LNXDrive daemon".
- [x] T043 [US5] Create `preferences/src/window.rs` — define `LnxdriveWindow` as `adw::ApplicationWindow` subclass. Implement `new(app: &LnxdriveApp)` setting default size from GSettings (window-width, window-height). Implement `show_onboarding()`: create `OnboardingView`, set as content. Implement `show_preferences()`: create `PreferencesDialog`, present. Save window size on close-request.
- [x] T044 [US5] Create `preferences/src/dbus_client.rs` — define `DbusClient` struct wrapping zbus `Connection`. Implement `DbusClient::new() -> Result<Self>`: connect to session bus, verify daemon is available. Async methods (all return `Result<T, DbusError>`): `is_authenticated() -> bool`, `start_auth() -> (String, String)` (auth_url, state), `complete_auth(code, state) -> bool`, `logout()`, `get_config() -> String` (YAML), `set_config(yaml: &str)`, `get_quota() -> (u64, u64)`, `get_account_info() -> HashMap<String, OwnedValue>`, `get_selected_folders() -> Vec<String>`, `set_selected_folders(folders: &[&str])`, `get_exclusion_patterns() -> Vec<String>`, `set_exclusion_patterns(patterns: &[&str])`, `get_remote_folder_tree() -> String` (JSON), `sync_now()`, `pause()`, `resume()`. Define `DbusError` enum mapping `org.enigmora.LNXDrive.Error.*` names to variants (NotRunning, NotAuthenticated, InvalidPath, InvalidConfig, NetworkError, InsufficientDiskSpace, FileInUse). Subscribe to signals: `FileStatusChanged`, `SyncProgress`, `AuthStateChanged`, `QuotaChanged`, `ConfigChanged` → expose as `glib::Signal` on a `glib::Object` subclass `DbusSignals` for GTK binding. All async operations spawned on `glib::MainContext::default()` (NOT tokio runtime). Add code comment documenting auth flow: "Daemon runs loopback HTTP server for OAuth2 callback. App calls StartAuth() → opens browser → daemon receives callback → emits AuthStateChanged. CompleteAuth() is for manual/CLI/GOA flows only." (ref pending issues U3, U4).

### Onboarding Module

- [x] T045 [US5] Create `preferences/src/onboarding/mod.rs` — declare `pub mod auth_page; pub mod folder_page; pub mod confirm_page;`. Define `OnboardingView` as `adw::NavigationView` subclass. Implement `new(dbus_client: DbusClient)`: create AuthPage, push as initial page. Define transient `OnboardingState` struct: `account_email: Option<String>`, `account_name: Option<String>`, `sync_root: Option<PathBuf>`. Implement `on_cancel()`: reset state to None for all fields, pop to AuthPage (FR-033).
- [x] T046 [US5] Create `preferences/src/onboarding/auth_page.rs` — define `AuthPage` as `adw::NavigationPage` subclass with title "Sign In". Build UI: `adw::StatusPage` with icon "dialog-password-symbolic", title "Sign in to OneDrive", description "You'll be redirected to Microsoft to authorize LNXDrive". "Sign In" `gtk::Button` (suggested-action style). On click: call `dbus_client.start_auth().await` → get `(auth_url, state)` → launch browser via `gio::AppInfo::launch_default_for_uri_async(&auth_url, ...)` → switch to waiting state: show spinner + "Waiting for authentication..." + Cancel button. Subscribe `AuthStateChanged` signal: on "authenticated" → store account info in OnboardingState → push FolderPage. On Cancel: call `on_cancel()` on OnboardingView. Handle error: show inline error banner "Authentication failed. Please try again."
- [x] T047 [US5] Create `preferences/src/onboarding/folder_page.rs` — define `FolderPage` as `adw::NavigationPage` with title "Choose Folder". Build UI: `adw::PreferencesGroup` with `adw::ActionRow` showing current path (default `~/OneDrive`). "Choose Folder..." button: open `gtk::FileDialog::new()` in select-folder mode, on response store path in OnboardingState. "Continue" button (suggested-action): validate path exists or can be created → push ConfirmPage. "Back" button: pop navigation.
- [x] T048 [US5] Create `preferences/src/onboarding/confirm_page.rs` — define `ConfirmPage` as `adw::NavigationPage` with title "Ready to Sync". Build UI: `adw::StatusPage` with icon "emblem-ok-symbolic", title "All Set!". `adw::PreferencesGroup` summary rows: "Account" → email from OnboardingState, "Sync Folder" → path from OnboardingState. "Start Syncing" button (suggested-action): call `dbus_client.set_config()` with sync_root YAML → call `dbus_client.sync_now()` (FR-034) → on success: switch window to preferences view via `window.show_preferences()`. "Back" button: pop to FolderPage. Handle D-Bus errors: show toast "Failed to start sync: {error}".

### Build Verification

- [x] T049 [US5] Verify `cargo check` succeeds in `preferences/` with all modules compiling. Create minimal `preferences/src/preferences/mod.rs` stub (`pub struct PreferencesDialog;` with placeholder `new()` / `present()`) so app.rs compiles. Verify `cargo run` with mock daemon (`--authenticated=false` flag) launches onboarding wizard.

**Checkpoint**: `cargo run` in preferences/ with mock daemon (unauthenticated) → wizard appears → auth opens browser → mock emits authenticated → folder selection → confirm starts sync → cancel discards state. **FR-031, FR-032, FR-033, FR-034 covered.**

---

## Stage 5: US3 — GNOME Shell Status Indicator (Priority: P2)

**Goal**: Persistent icon in Shell top bar + dropdown menu with sync progress, conflicts, quota, quick actions.

**Independent Test**: Copy extension to `~/.local/share/gnome-shell/extensions/` → `gnome-extensions enable lnxdrive-indicator@enigmora.com` → icon appears → start mock daemon → indicator reflects state → click menu → sections populated → daemon emits signals → updates within 3s (SC-003).

**Language**: GJS (ESM) | **Directory**: `shell-extension/lnxdrive-indicator@enigmora.com/`

**Pending Issues**: I5 (quota format), A2 (resource metrics), G1 (SC-008 ↔ FR-025 link)

### Extension Metadata & Skeleton

- [x] T050 [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/metadata.json` — `{ "uuid": "lnxdrive-indicator@enigmora.com", "name": "LNXDrive", "description": "Cloud sync status indicator for LNXDrive", "shell-version": ["45", "46", "47"], "version": 1, "url": "https://github.com/enigmora/lnxdrive-gnome", "settings-schema": "com.enigmora.LNXDrive.Indicator" }`
- [x] T051 [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/extension.js` — `import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';` Import `LnxdriveIndicator` from `./indicator.js`. Class `LnxdriveExtension extends Extension`: `enable()` → `this._indicator = new LnxdriveIndicator(this);` → `Main.panel.addToStatusArea('lnxdrive', this._indicator);`. `disable()` → `this._indicator?.destroy();` → `this._indicator = null;`. Do NOT import Gdk/Gtk/Adw (Shell extension lifecycle rule).

### D-Bus Proxy Wrappers

- [x] T052 [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/dbus.js` — define XML interface strings matching contracts for `.Sync` (methods: SyncNow, Pause, Resume; properties: SyncStatus, LastSyncTime, PendingChanges; signals: SyncStarted, SyncCompleted, SyncProgress, ConflictDetected), `.Status` (methods: GetQuota, GetAccountInfo; properties: ConnectionStatus; signals: QuotaChanged, ConnectionChanged), `.Manager` (methods: GetStatus; properties: Version, IsRunning). Create wrappers: `const SyncProxy = Gio.DBusProxy.makeProxyWrapper(SYNC_XML);`, same for StatusProxy, ManagerProxy. Export `async function createProxies()`: construct all 3 proxies async against bus name `org.enigmora.LNXDrive` object path `/org/enigmora/LNXDrive`, return `{ sync, status, manager }`. Catch construction errors (daemon not running) → return null proxies with logged warning.

### Indicator — Icon & State Machine

- [x] T053 [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/indicator.js` — `import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';` Import St, Clutter, GObject from gi. Import `{createProxies}` from `./dbus.js`, `{buildMenu}` from `./menuItems.js`. Define `LnxdriveIndicator extends PanelMenu.Button`: `_init(extension)` → `super._init(0.0, 'LNXDrive');` → create `St.Icon` with icon-name `'com.enigmora.LNXDrive-symbolic'` style-class `'system-status-icon'` → `this.add_child(icon);` → `this._proxies = null;` → `this._initProxies();`. `async _initProxies()` → `this._proxies = await createProxies();` → if null: set icon "offline" state, schedule retry in 5s → else: build menu via `buildMenu(this.menu, this._proxies)` → connect `this._proxies.sync` property change `SyncStatus` → `this._updateIconState(status)` → connect `notify::g-name-owner` on all proxies: on null → `this._onDaemonLost()`, on value → `this._onDaemonFound()` (FR-025, SC-008, ref pending issue G1: "reconnection must complete within 10s per SC-008"). Define `_updateIconState(status)`: remove all state CSS classes → add class based on status: idle→nothing, syncing→`'lnxdrive-syncing'`, paused→`'lnxdrive-paused'`, error→`'lnxdrive-error'`, offline→`'lnxdrive-offline'`. `_onDaemonLost()` → set "offline" state, show "Daemon not running" in menu. `_onDaemonFound()` → re-create proxies, rebuild menu, query current state. `destroy()` → disconnect ALL signal handler IDs stored in `this._signalIds` array, call `super.destroy()`.

### Menu Items Builder

- [x] T054 [P] [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/menuItems.js` — import PopupMenu from Shell resources. Export `function buildMenu(menu, proxies)` constructing sections: (1) **Sync Progress** section: `PopupMenu.PopupMenuItem` showing sync status text ("Idle" / "Syncing: filename (45%)..." / "Paused"), connect to `SyncProgress` signal → update with `file` + `current/total` percentage + `"${proxies.sync.PendingChanges} files pending"` (FR-010). (2) `PopupMenu.PopupSeparatorMenuItem`. (3) **Conflicts** section: `PopupMenu.PopupMenuItem` "No conflicts" initially, update count on `ConflictDetected` signal, connect `activate` → log/open conflict resolver placeholder (FR-010). (4) `PopupMenu.PopupSeparatorMenuItem`. (5) **Quota** section: `PopupMenu.PopupMenuItem` with custom child: `St.BoxLayout` containing `St.Label` "X.X GB / Y GB" + `St.Widget` styled as progress bar via `style: 'width: Xpx'` proportional fill (ref pending issue I5: use "X.X GB / Y GB" format + visual bar). Call `proxies.status.GetQuotaRemote()` on init, subscribe `QuotaChanged` signal for updates (FR-010). (6) `PopupMenu.PopupSeparatorMenuItem`. (7) **Actions** section: "Pause Sync" `PopupMenu.PopupMenuItem` → toggle Pause()/Resume(), update label to "Resume Sync" (FR-011). "Sync Now" `PopupMenu.PopupMenuItem` → call SyncNow() (FR-011). "Preferences" `PopupMenu.PopupMenuItem` → launch `com.enigmora.LNXDrive.Preferences` via `Gio.AppInfo.launch_default_for_uri()` or `Gio.DesktopAppInfo` (FR-011). Return array of signal handler IDs for cleanup.

### Preferences Entry Point (Minimal)

- [x] T055 [P] [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/prefs.js` — `import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';` Import Adw, Gtk from gi. `class LnxdrivePreferences extends ExtensionPreferences`: `fillPreferencesWindow(window)` → create `Adw.PreferencesPage` with single `Adw.PreferencesGroup` containing `Adw.ActionRow` title="Full Settings" subtitle="Open the LNXDrive preferences app" + button "Open Preferences" → on click launch `com.enigmora.LNXDrive.Preferences` desktop app. Add page to window.

### Stylesheet

- [x] T056 [P] [US3] Create `shell-extension/lnxdrive-indicator@enigmora.com/stylesheet.css` — `.lnxdrive-syncing { animation: lnxdrive-spin 2s linear infinite; } @keyframes lnxdrive-spin { to { rotation-angle: 360; } }` — `.lnxdrive-paused .system-status-icon { opacity: 0.5; }` — `.lnxdrive-error .system-status-icon { color: #e74c3c; }` — `.lnxdrive-offline .system-status-icon { opacity: 0.3; }` — `.lnxdrive-quota-bar { background-color: rgba(255,255,255,0.2); border-radius: 3px; height: 6px; }` — `.lnxdrive-quota-fill { background-color: #3584e4; border-radius: 3px; height: 6px; }` — `.lnxdrive-status-label { font-size: 0.9em; color: rgba(255,255,255,0.7); }`

**Checkpoint**: Extension installed + enabled → icon in top bar → mock daemon signals change icon state → click menu shows progress/conflicts/quota/actions → Pause/Resume/SyncNow work → daemon disconnect → "offline" state → daemon reconnect → recovers. **FR-009, FR-010, FR-011, FR-012, FR-025, FR-026 covered.**

---

## Stage 6: US4 — Preferences Panel (Priority: P2)

**Goal**: GTK4/libadwaita preferences with account info, selective sync tree, exclusion patterns, bandwidth limits, conflict policy.

**Independent Test**: `cargo run` with mock daemon (authenticated) → preferences panel opens → Account page shows mock data → Sync page shows folder tree → modify exclusion patterns → verify D-Bus calls in mock logs → change conflict policy → verify.

**Language**: Rust (gtk4-rs + libadwaita-rs) | **Directory**: `preferences/src/preferences/`

**Depends on**: Stage 4 (app scaffold: main.rs, app.rs, window.rs, dbus_client.rs)

**Pending Issues**: U6 (folder tree JSON schema), U5 (sync_on_startup)

### Preferences Module Structure

- [x] T057 [US4] Replace `preferences/src/preferences/mod.rs` stub (from T049) with full module: declare `pub mod account_page; pub mod sync_page; pub mod advanced_page; pub mod folder_tree;`. Define `PreferencesDialog` as `adw::PreferencesDialog` subclass. Implement `new(dbus_client: &DbusClient)` → create AccountPage, SyncPage, AdvancedPage → `self.add(account_page)`, `self.add(sync_page)`, `self.add(advanced_page)`. Implement `present(parent: &impl IsA<gtk::Widget>)`.

### Account Page

- [x] T058 [P] [US4] Create `preferences/src/preferences/account_page.rs` — define `AccountPage` as `adw::PreferencesPage` subclass, icon-name: "user-info-symbolic", title: "Account". Build: (1) `adw::PreferencesGroup` "OneDrive Account" with `adw::ActionRow` subtitle=email from `dbus_client.get_account_info()`, `adw::ActionRow` subtitle=display_name. (2) `adw::PreferencesGroup` "Storage" with `gtk::LevelBar` showing quota_used/quota_total from `dbus_client.get_quota()`, `gtk::Label` formatted "X.X GB of Y GB used". Subscribe `QuotaChanged` D-Bus signal → update bar + label. (3) `adw::PreferencesGroup` "Session" with `gtk::Button` "Sign Out" (destructive-action style) → on click: show confirmation `adw::AlertDialog` "Sign out of LNXDrive?" → on confirm: `dbus_client.logout()` → switch window to onboarding.

### Sync Page

- [x] T059 [P] [US4] Create `preferences/src/preferences/sync_page.rs` — define `SyncPage` as `adw::PreferencesPage` subclass, icon-name: "folder-symbolic", title: "Sync". Build: (1) `adw::PreferencesGroup` "Sync Options" with: `adw::SwitchRow` title="Automatic Sync" (FR-018), bind to sync_mode from `dbus_client.get_config()` (Automatic=on, Manual=off), on toggle: update config via `set_config()`. `adw::ComboRow` title="Conflict Resolution" (FR-016), model: `gtk::StringList` ["Always Ask", "Keep Local", "Keep Remote", "Keep Both"], set selected from config `conflict_policy`, on change: `set_config()`. `adw::SpinRow` title="Sync Interval (minutes)" adjustment: min=1 max=60 step=1 default=5, on change: `set_config()`. (2) `adw::PreferencesGroup` "Selective Sync" with subtitle "Choose which folders to sync" containing `FolderTree` widget (FR-014). Load all initial values from `dbus_client.get_config()`. Add 500ms debounce timer before sending config changes to avoid rapid D-Bus calls.

### Folder Tree — Selective Sync Widget

- [x] T060 [US4] Create `preferences/src/preferences/folder_tree.rs` — define `FolderNode` as `glib::Object` subclass with properties: `name: String`, `path: String`, `selected: bool`, `children_loaded: bool`. Define `FolderTree` as `gtk::Box` (vertical) containing `gtk::ScrolledWindow` → `gtk::ListView`. Use `gtk::TreeListModel` backed by `gio::ListStore` of `FolderNode`. Implement `gtk::SignalListItemFactory`: `setup` → create `gtk::Box` horizontal with `gtk::TreeExpander` + `gtk::CheckButton` + `gtk::Label`. `bind` → bind expander to `TreeListRow`, bind label to `FolderNode.name`, bind checkbox to `FolderNode.selected` bidirectional. On checkbox toggle: (a) if checking parent → propagate check to all children, (b) if unchecking last child → uncheck parent, (c) collect all selected paths → `dbus_client.set_selected_folders()`. Implement lazy loading: on `TreeListModel` `create_model` closure → if node not loaded: call `dbus_client.get_remote_folder_tree()` (ref pending issue U6: parse JSON `{"name":"str","path":"str","children":[...]}`) → create child ListStore → set `children_loaded=true`. On initial construction: load root-level folders from `dbus_client.get_selected_folders()` to set initial checkbox states.

### Advanced Page

- [x] T061 [P] [US4] Create `preferences/src/preferences/advanced_page.rs` — define `AdvancedPage` as `adw::PreferencesPage` subclass, icon-name: "preferences-other-symbolic", title: "Advanced". Build: (1) `adw::PreferencesGroup` "Exclusion Patterns" (FR-015) with `gtk::ListBox` selection=none: populate from `dbus_client.get_exclusion_patterns()`, each row = `adw::ActionRow` title=pattern + suffix `gtk::Button` icon "edit-delete-symbolic" → on click remove pattern. Below listbox: `gtk::Box` horizontal with `gtk::Entry` placeholder="*.tmp" + `gtk::Button` "Add" → on click: validate pattern, add to list, call `dbus_client.set_exclusion_patterns()`. (2) `adw::PreferencesGroup` "Bandwidth Limits" (FR-017) with `adw::SpinRow` title="Upload Limit (KB/s)" subtitle="0 = unlimited" adjustment: min=0 max=100000 step=100, load from config. `adw::SpinRow` title="Download Limit (KB/s)" subtitle="0 = unlimited" same range. On change: `set_config()` with updated bandwidth fields.

### Window Integration

- [x] T062 [US4] Update `preferences/src/window.rs` — replace stub call in `show_preferences()`: instantiate `preferences::PreferencesDialog::new(&dbus_client)` and call `dialog.present(self)`. Update `preferences/src/app.rs` if needed to pass `dbus_client` to window properly.

### Build Verification

- [x] T063 [US4] Verify `cargo build` succeeds with all preferences modules. Verify `cargo run` with mock daemon (`--authenticated`) opens preferences panel → Account page shows mock email/quota → Sync page shows folder tree loading from mock → Advanced page shows exclusion patterns. Modify a setting → verify mock daemon logs the D-Bus call.

**Checkpoint**: Full preferences panel functional with 3 pages, folder tree with lazy loading and checkbox propagation, all changes saved via D-Bus. **FR-013, FR-014, FR-015, FR-016, FR-017, FR-018 covered.**

---

## Stage 7: US6 — GNOME Online Accounts Provider (Priority: P3, Deferred)

**Goal**: Skeleton and documentation only. Full GOA provider implementation deferred.

- [x] T064 [US6] Create `goa-provider/README.md` — document planned GOA provider architecture: C shared library implementing `GoaProvider` GObject interface, OAuth2 via WebKitGTK embedded view, token handoff to daemon via `Auth.CompleteAuth()`, account lifecycle monitoring. List FR coverage: FR-019 (provider registration), FR-020 (OAuth2 PKCE), FR-021 (SSO), FR-022 (token refresh), FR-023 (account removal). Mark all as "P3 — not yet implemented".
- [x] T065 [P] [US6] Create `goa-provider/meson.build` — conditional build: `if get_option('enable_goa')` guarded, empty `shared_library()` target placeholder with TODO comments, `endif`

**Checkpoint**: GOA skeleton documented, build placeholder present (no functional code).

---

## Stage 8: Polish & Cross-Cutting Concerns

**Purpose**: Integration testing, i18n string extraction, terminology, documentation.

### Integration Tests

- [x] T066 Create `tests/test-nautilus-extension.py` — Python test script: (1) start mock-dbus-daemon.py as subprocess, (2) use `gi.repository.Gio` to create `GDBusProxy` for `.Files`, (3) call `GetFileStatus` for mock paths → verify expected statuses, (4) call `GetBatchFileStatus` → verify hash table format, (5) emit `FileStatusChanged` signal → verify observable (timing), (6) test error cases: call PinFile on non-existent path → verify error name. Run with `meson test`.
- [x] T067 [P] Create `tests/test-shell-extension.js` — GJS test script: (1) import dbus.js module, (2) verify `createProxies()` returns valid objects when mock daemon is running, (3) verify `createProxies()` handles daemon absence gracefully (returns null), (4) verify proxy signal subscription works, (5) basic lifecycle: create/destroy indicator (mock Shell environment if possible). Run with `gjs test-shell-extension.js`.

### i18n String Extraction

- [x] T068 [P] Mark all user-facing strings in `nautilus-extension/src/*.c` with `_()` gettext macro. Add `#include <glib/gi18n.h>` to each .c file. Strings to mark: menu item labels ("Keep Available Offline", "Free Up Space", "Sync Now", "Sync This Folder", "LNXDrive", "LNXDrive — Service not running"), error notification titles/bodies, column labels.
- [x] T069 [P] Mark all user-facing strings in `preferences/src/**/*.rs` with `gettextrs::gettext!()` macro. Strings: all button labels, page titles, group titles, row titles/subtitles, error messages, onboarding wizard text, status page descriptions, dialog messages.
- [x] T070 [P] Mark all user-facing strings in `shell-extension/**/*.js` with `extension.gettext()` calls. Strings: menu item labels ("Pause Sync", "Resume Sync", "Sync Now", "Preferences", "No conflicts"), status text, quota format string, error messages.
- [x] T071 Regenerate `po/lnxdrive-gnome.pot` by running: `xgettext` for C sources, `xtr` for Rust sources, `xgettext --language=JavaScript` for GJS sources. Merge into single .pot. Verify no duplicate msgids, all source strings captured.

### Terminology & Documentation

- [x] T072 [P] Add terminology glossary to `nautilus-extension/src/lnxdrive-dbus-client.c` (top-of-file comment block) and `preferences/src/dbus_client.rs` (module doc comment): "CloudOnly = 'cloud-only' (D-Bus string) = 'placeholder' (user-facing term). UnpinFile = unpin + dehydrate (makes file cloud-only, frees local space). PinFile = hydrate + pin (downloads file, keeps local)." (ref pending issues T1, T2)
- [x] T073 [P] Update `specs/001-gnome-integration/quickstart.md` — verify all build commands match final meson.build structure, update Cargo.toml dependency listing if changed, add mock daemon usage examples with actual CLI flags.

### Full Build Validation

- [x] T074 Run full build cycle: `meson setup builddir --prefix=$HOME/.local && meson compile -C builddir && meson install -C builddir`, verify: `liblnxdrive-nautilus.so` installed to nautilus extensions dir, Shell extension files installed to gnome-shell extensions dir, `lnxdrive-preferences` binary installed to bindir, all icons installed, desktop file installed, gschema compiled.
- [x] T075 Run `meson test -C builddir` executing test-nautilus-extension.py and test-shell-extension.js, verify all tests pass against mock daemon.

**Checkpoint**: Full build succeeds, all tests pass, i18n strings extracted, documentation accurate. All 37 FRs covered. All 8 edge cases handled. All 8 success criteria testable.

---

## Dependencies & Execution Order

### Stage Dependencies

```
Stage 1 (Setup) ──────────► Stage 2 (Foundation)
                                   │
                    ┌──────────────┼──────────────┐
                    │              │              │
                    ▼              ▼              ▼
          Stage 3 (US1+US2) Stage 4 (US5)  Stage 5 (US3)
          Nautilus [C]      Onboarding     Shell [GJS]
                    │       [Rust]              │
                    │              │              │
                    │              ▼              │
                    │       Stage 6 (US4)        │
                    │       Preferences          │
                    │       [Rust]               │
                    │              │              │
                    └──────┬───────┘              │
                           │                     │
                           ▼                     │
                    Stage 7 (US6, P3)            │
                    GOA [deferred]               │
                           │                     │
                           └─────────┬───────────┘
                                     │
                                     ▼
                             Stage 8 (Polish)
```

### Parallel Execution Strategy (3 Subagents)

After Stage 2 completes, launch **3 parallel subagents**:

| Subagent | Stage | Language | Directory | Tasks | Est. Files |
|----------|-------|----------|-----------|-------|------------|
| **Agent A** | Stage 3 | C | `nautilus-extension/` | T028–T040 (13 tasks) | 10 files |
| **Agent B** | Stage 4 → 6 | Rust | `preferences/` | T041–T049, T057–T063 (16 tasks) | 14 files |
| **Agent C** | Stage 5 | GJS | `shell-extension/` | T050–T056 (7 tasks) | 7 files |

**Convergence**: Once A, B, C complete → Stage 8 (any agent or new agent).

### Within-Stage Parallel Opportunities

| Stage | Parallel Groups | Details |
|-------|----------------|---------|
| **Stage 1** | T002 ∥ T003 ∥ T004 ∥ T005 ∥ T006 ∥ T007 | After T001 (top-level meson.build), all component build files are independent |
| **Stage 2** | Icons (T009–T017) ∥ i18n (T020–T023) ∥ Desktop (T024–T027) | Three independent groups. T008 (mock daemon) is sequential. T018–T019 sequential after icons. |
| **Stage 3** | T034–T035 (ColumnProvider) ∥ T028–T033 (DbusClient+InfoProvider) | ColumnProvider has no deps on DbusClient. MenuProvider (T036–T039) needs DbusClient. |
| **Stage 4** | T041–T044 sequential (scaffold) → T046 ∥ T047 (auth ∥ folder pages) | Pages independent but share OnboardingView (T045). T048 depends on both pages. |
| **Stage 5** | T050–T052 sequential (skeleton+dbus) → T054 ∥ T055 ∥ T056 (menu ∥ prefs ∥ css) | T053 (indicator) depends on T052 (dbus). Menu/prefs/css are independent files. |
| **Stage 6** | T058 ∥ T059 ∥ T061 (three pages parallel) | All pages independent. T060 (folder_tree) referenced by sync_page. T062 depends on all pages. |
| **Stage 8** | T066 ∥ T067 (tests) ∥ T068 ∥ T069 ∥ T070 (i18n per lang) ∥ T072 ∥ T073 | Mostly independent files. T071 depends on T068–T070. T074–T075 are final sequential. |

---

## Implementation Strategy

### MVP First (Stages 1 → 2 → 3)

1. Complete Stage 1: Setup → build files generate
2. Complete Stage 2: Foundation → mock daemon + icons ready
3. Complete Stage 3: US1+US2 → **MVP: Nautilus overlay icons + context menu**
4. **STOP & VALIDATE**: Test with mock daemon in real Nautilus

### Incremental Delivery

| Increment | Stages | Value Delivered |
|-----------|--------|-----------------|
| MVP | 1 + 2 + 3 | Nautilus overlay icons + context menu |
| +Onboarding | + 4 | First-run wizard works |
| +Indicator | + 5 | Shell status in top bar |
| +Preferences | + 6 | Full configuration panel |
| +Polish | + 8 | Tests, i18n, docs |

### Subagent Parallel Strategy

```
Timeline:  ═══════════════════════════════════════════►

All:       [Stage 1][Stage 2]
Agent A:                     [   Stage 3 (Nautilus/C)   ]
Agent B:                     [Stage 4 (Onboarding)][Stage 6 (Prefs)]
Agent C:                     [  Stage 5 (Shell/GJS)  ]
All:                                                     [Stage 8]
```

---

## Summary

| Metric | Value |
|--------|-------|
| **Total Tasks** | **75** |
| **Parallel-marked [P]** | **30** (40%) |
| Tasks per Stage | S1: 7, S2: 20, S3: 13, S4: 9, S5: 7, S6: 7, S7: 2, S8: 10 |
| Tasks per User Story | US1: 10, US2: 5, US3: 7, US4: 7, US5: 9, US6: 2 |
| FR Coverage | 37/37 (100%) |
| Pending issues referenced | 16/16 (all tagged in relevant tasks) |
| Languages | C (13 tasks), Rust (16 tasks), GJS (7 tasks), Python (1 task), Cross-cutting (10 tasks) |
| Max parallel subagents | **3** (after Stage 2) |

---

## Notes

- **[P]** = different files, no shared state — safe for parallel subagent execution
- **[USn]** label maps each task to its user story for FR traceability
- Each stage has a **Checkpoint** describing exactly how to verify completion
- Pending issues from `/speckit.analyze` are referenced inline (e.g., "ref pending issue I2")
- "Stage" nomenclature used per project convention (plan.md uses "Phase" for architecture)
- Commit after each task or logical group of tasks
