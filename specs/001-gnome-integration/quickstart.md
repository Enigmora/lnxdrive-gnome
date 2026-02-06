# Quickstart: GNOME Desktop Integration

**Branch**: `001-gnome-integration` | **Date**: 2026-02-05

---

## Prerequisites

### System Dependencies

```bash
# Fedora 40+ / GNOME 45+
sudo dnf install \
    meson ninja-build \
    gtk4-devel libadwaita-devel \
    nautilus-devel \
    glib2-devel gio-unix-devel \
    gettext-devel \
    rust cargo \
    gnome-shell

# Ubuntu 24.04+ / GNOME 46+
sudo apt install \
    meson ninja-build \
    libgtk-4-dev libadwaita-1-dev \
    libnautilus-extension-dev \
    libglib2.0-dev \
    gettext \
    rustc cargo \
    gnome-shell
```

### Rust Toolchain

```bash
# Minimum Rust 1.83
rustup update stable
```

### Daemon (mock or real)

The GNOME components communicate with `lnxdrive-daemon` via D-Bus. For development:

```bash
# Option A: Run the real daemon (requires lnxdrive core built)
lnxdrive-daemon &

# Option B: Use the D-Bus mock daemon (see testing section below)
python3 tests/mock-dbus-daemon.py --authenticated &
```

---

## Project Structure

```
lnxdrive-gnome/
├── meson.build                    # Top-level Meson build
├── meson_options.txt              # Build options
│
├── nautilus-extension/            # C shared library
│   ├── meson.build
│   ├── src/
│   │   ├── lnxdrive-extension.c  # Module entry points
│   │   ├── lnxdrive-info-provider.c
│   │   ├── lnxdrive-menu-provider.c
│   │   ├── lnxdrive-column-provider.c
│   │   └── lnxdrive-dbus-client.c
│   └── icons/                     # Emblem icons (SVG)
│       ├── lnxdrive-synced.svg
│       ├── lnxdrive-cloud-only.svg
│       ├── lnxdrive-syncing.svg
│       ├── lnxdrive-pending.svg
│       ├── lnxdrive-conflict.svg
│       ├── lnxdrive-error.svg
│       └── lnxdrive-unknown.svg
│
├── shell-extension/               # GJS GNOME Shell extension
│   └── lnxdrive-indicator@enigmora.com/
│       ├── extension.js
│       ├── metadata.json
│       ├── prefs.js
│       ├── stylesheet.css
│       ├── dbus.js
│       ├── indicator.js
│       └── menuItems.js
│
├── preferences/                   # Rust GTK4/libadwaita application
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── app.rs
│   │   ├── window.rs
│   │   ├── onboarding/
│   │   │   ├── mod.rs
│   │   │   ├── auth_page.rs
│   │   │   ├── folder_page.rs
│   │   │   └── confirm_page.rs
│   │   ├── preferences/
│   │   │   ├── mod.rs
│   │   │   ├── account_page.rs
│   │   │   ├── sync_page.rs
│   │   │   ├── advanced_page.rs
│   │   │   └── folder_tree.rs
│   │   └── dbus_client.rs
│   └── data/
│       ├── com.enigmora.LNXDrive.Preferences.desktop.in
│       ├── com.enigmora.LNXDrive.Preferences.metainfo.xml.in
│       └── com.enigmora.LNXDrive.Preferences.gschema.xml
│
├── po/                            # Translations
│   ├── POTFILES.in
│   ├── LINGUAS
│   └── lnxdrive-gnome.pot
│
├── data/                          # Shared data files
│   └── icons/
│       └── hicolor/
│           ├── scalable/apps/com.enigmora.LNXDrive.svg
│           └── symbolic/apps/com.enigmora.LNXDrive-symbolic.svg
│
└── tests/                         # Integration tests
    ├── test-nautilus-extension.py
    ├── test-shell-extension.js
    └── mock-dbus-daemon.py
```

---

## Build & Run

### Full Build

```bash
meson setup builddir --prefix=$HOME/.local
meson compile -C builddir
meson install -C builddir
```

### Component-Specific

```bash
# Nautilus extension only
meson compile -C builddir nautilus-extension

# Rust preferences app only
cd preferences && cargo build

# Shell extension (install to user dir)
cp -r shell-extension/lnxdrive-indicator@enigmora.com \
    ~/.local/share/gnome-shell/extensions/
```

### Test Nautilus Extension

```bash
# Restart Nautilus to load the extension
nautilus -q && nautilus &
```

### Test Shell Extension

```bash
# Enable the extension
gnome-extensions enable lnxdrive-indicator@enigmora.com

# View logs
journalctl -f -o cat /usr/bin/gnome-shell
```

### Test Preferences App

```bash
cd preferences && cargo run
```

---

## Development Workflow

1. **Start the D-Bus mock daemon** (provides fake sync data):
   `python3 tests/mock-dbus-daemon.py --authenticated &`
2. **Build and install components** to `~/.local`
3. **Restart Nautilus** to test overlay icons and context menu
4. **Enable Shell extension** to test indicator
5. **Run preferences app** to test configuration UI
6. **Run integration tests**:
   `python3 tests/test-nautilus-extension.py` and `gjs tests/test-shell-extension.js`

### Useful Commands

```bash
# Monitor D-Bus traffic
dbus-monitor --session "interface='org.enigmora.LNXDrive.Files'"

# Call D-Bus methods manually
gdbus call --session \
    --dest org.enigmora.LNXDrive \
    --object-path /org/enigmora/LNXDrive \
    --method org.enigmora.LNXDrive.Files.GetFileStatus \
    "/home/user/OneDrive/document.pdf"

# Check if daemon is running
gdbus introspect --session \
    --dest org.enigmora.LNXDrive \
    --object-path /org/enigmora/LNXDrive
```

---

## Key Dependencies (Cargo.toml for preferences/)

```toml
[dependencies]
gtk4 = { version = "0.9", features = ["v4_14"] }
libadwaita = { version = "0.7", features = ["v1_6"] }
zbus = "5"
# lnxdrive-ipc = { git = "https://github.com/enigmora/lnxdrive.git", package = "lnxdrive-ipc" }
oauth2 = "5"
gettext-rs = { version = "0.7", features = ["gettext-system"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt"] }
futures-util = "0.3"
```

> **Note**: The crate is named `gettext-rs` in Cargo.toml but is imported as
> `gettextrs` in Rust code (`use gettextrs::gettext;`). The `lnxdrive-ipc`
> dependency is currently commented out pending publication of the IPC crate.

---

## Mock D-Bus Daemon

The mock daemon at `tests/mock-dbus-daemon.py` simulates all 6 D-Bus interfaces
of the LNXDrive daemon. It requires `pip install dbus-next`.

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--authenticated` | false | Start with auth state = authenticated |
| `--signal-interval N` | 5.0 | Seconds between periodic signal emissions |
| `--sync-root PATH` | `~/OneDrive` | Mock sync root directory |

### Usage Examples

```bash
# Basic: unauthenticated, default sync root, periodic signals every 5s
python3 tests/mock-dbus-daemon.py

# Authenticated with custom sync root (useful for testing preferences)
python3 tests/mock-dbus-daemon.py --authenticated --sync-root /tmp/test-onedrive

# No periodic signals (quieter for integration tests)
python3 tests/mock-dbus-daemon.py --authenticated --signal-interval 999

# Run integration tests against the mock daemon
python3 tests/mock-dbus-daemon.py --authenticated --signal-interval 999 &
python3 tests/test-nautilus-extension.py
gjs tests/test-shell-extension.js

# Test shell extension graceful degradation (no daemon)
gjs tests/test-shell-extension.js --no-daemon
```

### Hardcoded File Statuses

The mock daemon provides predefined statuses relative to the sync root:

| Relative Path | Status |
|---------------|--------|
| `document.pdf` | synced |
| `photos/` | cloud-only |
| `report.docx` | syncing |
| `budget.xlsx` | conflict |
| `notes.txt` | synced |
| `presentation.pptx` | pending |
| `archive.zip` | excluded |
| `shared/team-notes.docx` | error |
