#!/usr/bin/env python3
"""
Mock D-Bus Daemon for LNXDrive GNOME Integration Testing.

Implements all 6 D-Bus interfaces of org.enigmora.LNXDrive on the session bus:
  - org.enigmora.LNXDrive.Files
  - org.enigmora.LNXDrive.Sync
  - org.enigmora.LNXDrive.Status
  - org.enigmora.LNXDrive.Manager
  - org.enigmora.LNXDrive.Settings
  - org.enigmora.LNXDrive.Auth

Usage:
    python3 mock-dbus-daemon.py [--authenticated] [--signal-interval N] [--sync-root PATH]

Requirements:
    pip install dbus-next
"""

from __future__ import annotations

import argparse
import asyncio
import json
import logging
import os
import signal
import time
from pathlib import Path
from typing import Any

from dbus_next import Variant
from dbus_next.aio import MessageBus
from dbus_next.service import ServiceInterface, method, dbus_property, signal as dbus_signal

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger("lnxdrive-mock")

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
BUS_NAME = "org.enigmora.LNXDrive"
OBJECT_PATH = "/org/enigmora/LNXDrive"


# ===================================================================
# 1. org.enigmora.LNXDrive.Files
# ===================================================================
class FilesInterface(ServiceInterface):
    """Mock implementation of org.enigmora.LNXDrive.Files."""

    def __init__(self, sync_root: str) -> None:
        super().__init__("org.enigmora.LNXDrive.Files")
        self._sync_root = sync_root

        # Hardcoded file statuses keyed by path relative to sync_root.
        self._statuses: dict[str, str] = {
            "document.pdf": "synced",
            "photos/": "cloud-only",
            "photos/vacation/": "cloud-only",
            "photos/vacation/beach.jpg": "cloud-only",
            "report.docx": "syncing",
            "budget.xlsx": "conflict",
            "notes.txt": "synced",
            "presentation.pptx": "pending",
            "archive.zip": "excluded",
            "projects/": "synced",
            "projects/readme.md": "synced",
            "projects/src/main.rs": "synced",
            "shared/team-notes.docx": "error",
        }

    # -- helpers ----------------------------------------------------------

    def _relative_path(self, path: str) -> str:
        """Normalise an absolute path to a sync-root-relative key."""
        try:
            return str(Path(path).relative_to(self._sync_root))
        except ValueError:
            return path

    def _status_list(self) -> list[str]:
        """Return the list of known relative paths."""
        return list(self._statuses.keys())

    # -- methods ----------------------------------------------------------

    @method()
    def GetFileStatus(self, path: "s") -> "s":
        rel = self._relative_path(path)
        status = self._statuses.get(rel, "unknown")
        log.info("Files.GetFileStatus(%s) -> %s", path, status)
        return status

    @method()
    def GetBatchFileStatus(self, paths: "as") -> "a{ss}":
        result: dict[str, str] = {}
        for p in paths:
            rel = self._relative_path(p)
            result[p] = self._statuses.get(rel, "unknown")
        log.info("Files.GetBatchFileStatus(%d paths)", len(paths))
        return result

    @method()
    def PinFile(self, path: "s") -> None:
        rel = self._relative_path(path)
        log.info("Files.PinFile(%s) — pinning (was %s)", path, self._statuses.get(rel, "unknown"))
        self._statuses[rel] = "synced"
        self.FileStatusChanged(path, "synced")

    @method()
    def UnpinFile(self, path: "s") -> None:
        rel = self._relative_path(path)
        log.info("Files.UnpinFile(%s) — unpinning (was %s)", path, self._statuses.get(rel, "unknown"))
        self._statuses[rel] = "cloud-only"
        self.FileStatusChanged(path, "cloud-only")

    @method()
    def SyncPath(self, path: "s") -> None:
        rel = self._relative_path(path)
        log.info("Files.SyncPath(%s) — triggering sync", path)
        self._statuses[rel] = "syncing"
        self.FileStatusChanged(path, "syncing")

    @method()
    def GetConflicts(self) -> "as":
        conflicts = [
            os.path.join(self._sync_root, k)
            for k, v in self._statuses.items()
            if v == "conflict"
        ]
        log.info("Files.GetConflicts() -> %d conflicts", len(conflicts))
        return conflicts

    # -- signals ----------------------------------------------------------

    @dbus_signal()
    def FileStatusChanged(self, path, status) -> "ss":
        return [path, status]


# ===================================================================
# 2. org.enigmora.LNXDrive.Sync
# ===================================================================
class SyncInterface(ServiceInterface):
    """Mock implementation of org.enigmora.LNXDrive.Sync."""

    def __init__(self) -> None:
        super().__init__("org.enigmora.LNXDrive.Sync")
        self._sync_status: str = "idle"
        self._last_sync_time: int = int(time.time()) - 300  # 5 min ago
        self._pending_changes: int = 7
        self._syncing_task: asyncio.Task[None] | None = None

    # -- properties -------------------------------------------------------

    @dbus_property()
    def SyncStatus(self) -> "s":
        return self._sync_status

    @dbus_property()
    def LastSyncTime(self) -> "x":
        return self._last_sync_time

    @dbus_property()
    def PendingChanges(self) -> "u":
        return self._pending_changes

    # -- methods ----------------------------------------------------------

    @method()
    def SyncNow(self) -> None:
        log.info("Sync.SyncNow() — starting sync cycle")
        if self._sync_status == "syncing":
            log.warning("Sync.SyncNow() — already syncing, ignoring")
            return
        self._sync_status = "syncing"
        self.emit_properties_changed({"SyncStatus": self._sync_status})
        self.SyncStarted()
        # Schedule the simulated sync in the background.
        loop = asyncio.get_event_loop()
        self._syncing_task = loop.create_task(self._simulate_sync())

    @method()
    def Pause(self) -> None:
        log.info("Sync.Pause()")
        if self._syncing_task and not self._syncing_task.done():
            self._syncing_task.cancel()
            self._syncing_task = None
        self._sync_status = "paused"
        self.emit_properties_changed({"SyncStatus": self._sync_status})

    @method()
    def Resume(self) -> None:
        log.info("Sync.Resume()")
        if self._sync_status == "paused":
            self._sync_status = "idle"
            self.emit_properties_changed({"SyncStatus": self._sync_status})
            log.info("Sync.Resume() — status set to idle")

    # -- signals ----------------------------------------------------------

    @dbus_signal()
    def SyncStarted(self):
        pass

    @dbus_signal()
    def SyncCompleted(self, files_synced, errors) -> "uu":
        return [files_synced, errors]

    @dbus_signal()
    def SyncProgress(self, file, current, total) -> "suu":
        return [file, current, total]

    @dbus_signal()
    def ConflictDetected(self, path, conflict_type) -> "ss":
        return [path, conflict_type]

    # -- internal ---------------------------------------------------------

    async def _simulate_sync(self) -> None:
        """Simulate a sync cycle with progress updates."""
        mock_files = [
            "document.pdf",
            "notes.txt",
            "presentation.pptx",
            "projects/readme.md",
            "projects/src/main.rs",
        ]
        total = len(mock_files)

        try:
            for idx, filename in enumerate(mock_files, start=1):
                await asyncio.sleep(0.8)
                self._pending_changes = max(0, self._pending_changes - 1)
                self.emit_properties_changed({"PendingChanges": self._pending_changes})
                self.SyncProgress(filename, idx, total)
                log.info("Sync.SyncProgress(%s, %d/%d)", filename, idx, total)

            # Finished
            self._sync_status = "idle"
            self._last_sync_time = int(time.time())
            self._pending_changes = 0
            self.emit_properties_changed({
                "SyncStatus": self._sync_status,
                "LastSyncTime": self._last_sync_time,
                "PendingChanges": self._pending_changes,
            })
            self.SyncCompleted(total, 0)
            log.info("Sync.SyncCompleted(files_synced=%d, errors=0)", total)
        except asyncio.CancelledError:
            log.info("Sync cycle cancelled (pause/stop)")


# ===================================================================
# 3. org.enigmora.LNXDrive.Status
# ===================================================================
class StatusInterface(ServiceInterface):
    """Mock implementation of org.enigmora.LNXDrive.Status."""

    def __init__(self) -> None:
        super().__init__("org.enigmora.LNXDrive.Status")
        self._connection_status: str = "online"
        self._used: int = 5_368_709_120   # 5 GB
        self._total: int = 16_106_127_360  # ~15 GB

    # -- properties -------------------------------------------------------

    @dbus_property()
    def ConnectionStatus(self) -> "s":
        return self._connection_status

    # -- methods ----------------------------------------------------------

    @method()
    def GetQuota(self) -> "tt":
        log.info("Status.GetQuota() -> (%d, %d)", self._used, self._total)
        return [self._used, self._total]

    @method()
    def GetAccountInfo(self) -> "a{sv}":
        info: dict[str, Any] = {
            "email": Variant("s", "user@example.com"),
            "display_name": Variant("s", "Test User"),
            "provider": Variant("s", "onedrive"),
        }
        log.info("Status.GetAccountInfo() -> %s", {k: v.value for k, v in info.items()})
        return info

    # -- signals ----------------------------------------------------------

    @dbus_signal()
    def QuotaChanged(self, used, total) -> "tt":
        return [used, total]

    @dbus_signal()
    def ConnectionChanged(self, status) -> "s":
        return status


# ===================================================================
# 4. org.enigmora.LNXDrive.Manager
# ===================================================================
class ManagerInterface(ServiceInterface):
    """Mock implementation of org.enigmora.LNXDrive.Manager."""

    def __init__(self) -> None:
        super().__init__("org.enigmora.LNXDrive.Manager")
        self._is_running: bool = True
        self._version: str = "0.1.0-mock"

    # -- properties -------------------------------------------------------

    @dbus_property()
    def Version(self) -> "s":
        return self._version

    @dbus_property()
    def IsRunning(self) -> "b":
        return self._is_running

    # -- methods ----------------------------------------------------------

    @method()
    def Start(self) -> None:
        log.info("Manager.Start()")
        self._is_running = True
        self.emit_properties_changed({"IsRunning": self._is_running})

    @method()
    def Stop(self) -> None:
        log.info("Manager.Stop()")
        self._is_running = False
        self.emit_properties_changed({"IsRunning": self._is_running})

    @method()
    def Restart(self) -> None:
        log.info("Manager.Restart()")
        self._is_running = False
        self.emit_properties_changed({"IsRunning": self._is_running})
        self._is_running = True
        self.emit_properties_changed({"IsRunning": self._is_running})

    @method()
    def GetStatus(self) -> "s":
        status = "running" if self._is_running else "stopped"
        log.info("Manager.GetStatus() -> %s", status)
        return status


# ===================================================================
# 5. org.enigmora.LNXDrive.Settings
# ===================================================================

_DEFAULT_CONFIG_YAML = """\
sync_root: ~/OneDrive
sync_mode: hybrid
conflict_policy: rename_local
bandwidth:
  upload_limit_kbps: 0
  download_limit_kbps: 0
notifications:
  enabled: true
  sync_complete: true
  conflict: true
  errors: true
logging:
  level: info
  file: ~/.local/share/lnxdrive/lnxdrive.log
"""

_REMOTE_FOLDER_TREE = json.dumps(
    {
        "name": "root",
        "path": "/",
        "children": [
            {"name": "Documents", "path": "/Documents", "children": []},
            {
                "name": "Photos",
                "path": "/Photos",
                "children": [
                    {"name": "Vacation", "path": "/Photos/Vacation", "children": []},
                ],
            },
            {"name": "Projects", "path": "/Projects", "children": []},
        ],
    },
    indent=None,
)


class SettingsInterface(ServiceInterface):
    """Mock implementation of org.enigmora.LNXDrive.Settings."""

    def __init__(self, sync_root: str) -> None:
        super().__init__("org.enigmora.LNXDrive.Settings")
        self._config_yaml: str = _DEFAULT_CONFIG_YAML.replace("~/OneDrive", sync_root)
        self._selected_folders: list[str] = ["/Documents", "/Photos", "/Projects"]
        self._exclusion_patterns: list[str] = ["*.tmp", "~$*", ".~lock.*", "Thumbs.db"]

    # -- methods ----------------------------------------------------------

    @method()
    def GetConfig(self) -> "s":
        log.info("Settings.GetConfig()")
        return self._config_yaml

    @method()
    def SetConfig(self, yaml_str: "s") -> None:
        log.info("Settings.SetConfig(<yaml len=%d>)", len(yaml_str))
        self._config_yaml = yaml_str
        self.ConfigChanged("config")

    @method()
    def GetSelectedFolders(self) -> "as":
        log.info("Settings.GetSelectedFolders() -> %s", self._selected_folders)
        return self._selected_folders

    @method()
    def SetSelectedFolders(self, folders: "as") -> None:
        log.info("Settings.SetSelectedFolders(%s)", folders)
        self._selected_folders = list(folders)
        self.ConfigChanged("selected_folders")

    @method()
    def GetExclusionPatterns(self) -> "as":
        log.info("Settings.GetExclusionPatterns() -> %s", self._exclusion_patterns)
        return self._exclusion_patterns

    @method()
    def SetExclusionPatterns(self, patterns: "as") -> None:
        log.info("Settings.SetExclusionPatterns(%s)", patterns)
        self._exclusion_patterns = list(patterns)
        self.ConfigChanged("exclusion_patterns")

    @method()
    def GetRemoteFolderTree(self) -> "s":
        log.info("Settings.GetRemoteFolderTree()")
        return _REMOTE_FOLDER_TREE

    # -- signals ----------------------------------------------------------

    @dbus_signal()
    def ConfigChanged(self, key) -> "s":
        return key


# ===================================================================
# 6. org.enigmora.LNXDrive.Auth
# ===================================================================
class AuthInterface(ServiceInterface):
    """Mock implementation of org.enigmora.LNXDrive.Auth."""

    def __init__(self, authenticated: bool) -> None:
        super().__init__("org.enigmora.LNXDrive.Auth")
        self._authenticated: bool = authenticated

    # -- methods ----------------------------------------------------------

    @method()
    def StartAuth(self) -> "ss":
        auth_url = "https://login.microsoftonline.com/mock-auth?state=mock123"
        state = "mock123"
        log.info("Auth.StartAuth() -> (%s, %s)", auth_url, state)
        return [auth_url, state]

    @method()
    def CompleteAuth(self, code: "s", state: "s") -> "b":
        log.info("Auth.CompleteAuth(code=%s, state=%s) -> true", code, state)
        self._authenticated = True
        self.AuthStateChanged("authenticated")
        return True

    @method()
    def IsAuthenticated(self) -> "b":
        log.info("Auth.IsAuthenticated() -> %s", self._authenticated)
        return self._authenticated

    @method()
    def Logout(self) -> None:
        log.info("Auth.Logout()")
        self._authenticated = False
        self.AuthStateChanged("disconnected")

    # -- signals ----------------------------------------------------------

    @dbus_signal()
    def AuthStateChanged(self, state) -> "s":
        return state


# ===================================================================
# Periodic signal emitter
# ===================================================================
class PeriodicEmitter:
    """Emits periodic mock signals to simulate daemon activity."""

    def __init__(
        self,
        files_iface: FilesInterface,
        sync_iface: SyncInterface,
        status_iface: StatusInterface,
        interval: float,
        sync_root: str,
    ) -> None:
        self._files = files_iface
        self._sync = sync_iface
        self._status = status_iface
        self._interval = interval
        self._sync_root = sync_root
        self._task: asyncio.Task[None] | None = None
        self._tick: int = 0

    def start(self) -> None:
        loop = asyncio.get_event_loop()
        self._task = loop.create_task(self._run())

    async def stop(self) -> None:
        if self._task and not self._task.done():
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass

    async def _run(self) -> None:
        file_keys = list(self._files._statuses.keys())
        try:
            while True:
                await asyncio.sleep(self._interval)
                self._tick += 1

                # Cycle through file status changes.
                idx = self._tick % len(file_keys)
                rel_path = file_keys[idx]
                abs_path = os.path.join(self._sync_root, rel_path)
                current_status = self._files._statuses[rel_path]
                self._files.FileStatusChanged(abs_path, current_status)
                log.info(
                    "[periodic] FileStatusChanged(%s, %s)",
                    rel_path,
                    current_status,
                )

                # Emit SyncProgress if currently syncing.
                if self._sync._sync_status == "syncing":
                    self._sync.SyncProgress(
                        rel_path,
                        (self._tick % 5) + 1,
                        5,
                    )
                    log.info("[periodic] SyncProgress(%s)", rel_path)

                # Emit QuotaChanged every 6th tick (simulate slow drift).
                if self._tick % 6 == 0:
                    noise = (self._tick * 1_048_576) % 104_857_600  # up to 100 MB drift
                    used = self._status._used + noise
                    self._status.QuotaChanged(used, self._status._total)
                    log.info("[periodic] QuotaChanged(used=%d)", used)

        except asyncio.CancelledError:
            log.info("[periodic] Emitter stopped")


# ===================================================================
# Main entry point
# ===================================================================
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Mock D-Bus daemon for LNXDrive GNOME integration testing.",
    )
    parser.add_argument(
        "--authenticated",
        action="store_true",
        default=False,
        help="Start with authentication state set to true (default: false).",
    )
    parser.add_argument(
        "--signal-interval",
        type=float,
        default=5.0,
        metavar="N",
        help="Seconds between periodic signal emissions (default: 5).",
    )
    parser.add_argument(
        "--sync-root",
        type=str,
        default=os.path.expanduser("~/OneDrive"),
        metavar="PATH",
        help="Mock sync root path (default: ~/OneDrive).",
    )
    return parser.parse_args()


async def run(args: argparse.Namespace) -> None:
    stop_event = asyncio.Event()

    # Register signal handlers for graceful shutdown.
    loop = asyncio.get_event_loop()
    for sig in (signal.SIGINT, signal.SIGTERM):
        loop.add_signal_handler(sig, stop_event.set)

    log.info("Connecting to session bus...")
    bus = await MessageBus().connect()

    # Instantiate all interfaces.
    files_iface = FilesInterface(sync_root=args.sync_root)
    sync_iface = SyncInterface()
    status_iface = StatusInterface()
    manager_iface = ManagerInterface()
    settings_iface = SettingsInterface(sync_root=args.sync_root)
    auth_iface = AuthInterface(authenticated=args.authenticated)

    # Export all interfaces on the same object path.
    bus.export(OBJECT_PATH, files_iface)
    bus.export(OBJECT_PATH, sync_iface)
    bus.export(OBJECT_PATH, status_iface)
    bus.export(OBJECT_PATH, manager_iface)
    bus.export(OBJECT_PATH, settings_iface)
    bus.export(OBJECT_PATH, auth_iface)

    # Acquire the well-known bus name.
    await bus.request_name(BUS_NAME)

    log.info("=" * 60)
    log.info("LNXDrive Mock D-Bus Daemon is running")
    log.info("  Bus name:   %s", BUS_NAME)
    log.info("  Object:     %s", OBJECT_PATH)
    log.info("  Sync root:  %s", args.sync_root)
    log.info("  Auth state: %s", "authenticated" if args.authenticated else "not authenticated")
    log.info("  Signal interval: %.1fs", args.signal_interval)
    log.info("=" * 60)
    log.info("Interfaces:")
    log.info("  - org.enigmora.LNXDrive.Files")
    log.info("  - org.enigmora.LNXDrive.Sync")
    log.info("  - org.enigmora.LNXDrive.Status")
    log.info("  - org.enigmora.LNXDrive.Manager")
    log.info("  - org.enigmora.LNXDrive.Settings")
    log.info("  - org.enigmora.LNXDrive.Auth")
    log.info("Press Ctrl+C to stop.")

    # Start the periodic emitter.
    emitter = PeriodicEmitter(
        files_iface=files_iface,
        sync_iface=sync_iface,
        status_iface=status_iface,
        interval=args.signal_interval,
        sync_root=args.sync_root,
    )
    emitter.start()

    # Wait until a termination signal is received.
    await stop_event.wait()

    log.info("Shutting down...")
    await emitter.stop()
    bus.disconnect()
    log.info("Mock daemon stopped.")


def main() -> None:
    args = parse_args()
    try:
        asyncio.run(run(args))
    except KeyboardInterrupt:
        # Fallback in case the signal handler did not fire (e.g. Windows).
        log.info("Interrupted — exiting.")


if __name__ == "__main__":
    main()
