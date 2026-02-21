#!/usr/bin/env python3
"""
Integration tests for the LNXDrive Nautilus extension D-Bus client.

Starts the mock D-Bus daemon as a subprocess, then uses gi.repository.Gio to
create GDBusProxy objects and exercise the com.enigmora.LNXDrive.Files interface.

Usage:
    python3 tests/test-nautilus-extension.py

Requirements:
    - pip install dbus-next   (for the mock daemon)
    - gi.repository (PyGObject) must be available
"""

from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
import unittest
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
gi.require_version("GLib", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

# ---------------------------------------------------------------------------
# Constants matching the mock daemon
# ---------------------------------------------------------------------------
BUS_NAME = "com.enigmora.LNXDrive"
OBJECT_PATH = "/com/enigmora/LNXDrive"
IFACE_FILES = "com.enigmora.LNXDrive.Files"
IFACE_SETTINGS = "com.enigmora.LNXDrive.Settings"
IFACE_SYNC = "com.enigmora.LNXDrive.Sync"
IFACE_STATUS = "com.enigmora.LNXDrive.Status"

# Path to the mock daemon script (relative to this test file)
TESTS_DIR = Path(__file__).resolve().parent
MOCK_DAEMON = TESTS_DIR / "mock-dbus-daemon.py"

# Default sync root used by the mock daemon
DEFAULT_SYNC_ROOT = os.path.expanduser("~/OneDrive")


class NautilusExtensionDbusTest(unittest.TestCase):
    """Tests for the LNXDrive D-Bus Files interface via the mock daemon."""

    _daemon_proc: subprocess.Popen | None = None
    _files_proxy: Gio.DBusProxy | None = None

    @classmethod
    def setUpClass(cls) -> None:
        """Start the mock D-Bus daemon and wait for it to register on the bus."""
        if not MOCK_DAEMON.exists():
            raise FileNotFoundError(
                f"Mock daemon not found at {MOCK_DAEMON}. "
                "Run this test from the project root."
            )

        # Use a custom sync root in /tmp so paths are predictable
        cls._sync_root = os.path.join("/tmp", "lnxdrive-test-sync-root")

        cls._daemon_proc = subprocess.Popen(
            [
                sys.executable,
                str(MOCK_DAEMON),
                "--authenticated",
                "--signal-interval", "999",  # effectively disable periodic signals
                "--sync-root", cls._sync_root,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        # Wait for the daemon to appear on the session bus
        bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        deadline = time.monotonic() + 10  # 10 second timeout
        while time.monotonic() < deadline:
            try:
                result = bus.call_sync(
                    "org.freedesktop.DBus",
                    "/org/freedesktop/DBus",
                    "org.freedesktop.DBus",
                    "NameHasOwner",
                    GLib.Variant("(s)", (BUS_NAME,)),
                    GLib.VariantType.new("(b)"),
                    Gio.DBusCallFlags.NONE,
                    5000,
                    None,
                )
                has_owner = result.get_child_value(0).get_boolean()
                if has_owner:
                    break
            except GLib.Error:
                pass
            time.sleep(0.2)
        else:
            cls._kill_daemon()
            raise RuntimeError(
                "Mock D-Bus daemon did not appear on the session bus within 10s. "
                "Check that dbus-next is installed and a session bus is available."
            )

        # Create proxies for all interfaces
        cls._files_proxy = Gio.DBusProxy.new_for_bus_sync(
            Gio.BusType.SESSION,
            Gio.DBusProxyFlags.NONE,
            None,  # GDBusInterfaceInfo
            BUS_NAME,
            OBJECT_PATH,
            IFACE_FILES,
            None,  # GCancellable
        )
        cls._settings_proxy = Gio.DBusProxy.new_for_bus_sync(
            Gio.BusType.SESSION,
            Gio.DBusProxyFlags.NONE,
            None,
            BUS_NAME,
            OBJECT_PATH,
            IFACE_SETTINGS,
            None,
        )
        cls._sync_proxy = Gio.DBusProxy.new_for_bus_sync(
            Gio.BusType.SESSION,
            Gio.DBusProxyFlags.NONE,
            None,
            BUS_NAME,
            OBJECT_PATH,
            IFACE_SYNC,
            None,
        )
        cls._status_proxy = Gio.DBusProxy.new_for_bus_sync(
            Gio.BusType.SESSION,
            Gio.DBusProxyFlags.NONE,
            None,
            BUS_NAME,
            OBJECT_PATH,
            IFACE_STATUS,
            None,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        """Shut down the mock daemon."""
        cls._kill_daemon()

    @classmethod
    def _kill_daemon(cls) -> None:
        if cls._daemon_proc is not None:
            try:
                cls._daemon_proc.send_signal(signal.SIGTERM)
                cls._daemon_proc.wait(timeout=5)
            except (subprocess.TimeoutExpired, OSError):
                cls._daemon_proc.kill()
                cls._daemon_proc.wait(timeout=5)
            finally:
                cls._daemon_proc = None

    # ----- T066: GetFileStatus tests -----------------------------------------

    def test_get_file_status_synced(self) -> None:
        """GetFileStatus returns 'synced' for a known synced file."""
        path = os.path.join(self._sync_root, "document.pdf")
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "synced", f"Expected 'synced' for {path}, got '{status}'")

    def test_get_file_status_cloud_only(self) -> None:
        """GetFileStatus returns 'cloud-only' for a cloud-only directory."""
        path = os.path.join(self._sync_root, "photos/")
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "cloud-only", f"Expected 'cloud-only' for {path}, got '{status}'")

    def test_get_file_status_syncing(self) -> None:
        """GetFileStatus returns 'syncing' for a file being synced."""
        path = os.path.join(self._sync_root, "report.docx")
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "syncing", f"Expected 'syncing' for {path}, got '{status}'")

    def test_get_file_status_conflict(self) -> None:
        """GetFileStatus returns 'conflict' for a conflicted file."""
        path = os.path.join(self._sync_root, "budget.xlsx")
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "conflict", f"Expected 'conflict' for {path}, got '{status}'")

    def test_get_file_status_unknown_path(self) -> None:
        """GetFileStatus returns 'unknown' for a path not known to the daemon."""
        path = os.path.join(self._sync_root, "nonexistent-file.xyz")
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "unknown", f"Expected 'unknown' for {path}, got '{status}'")

    # ----- T066: GetBatchFileStatus tests ------------------------------------

    def test_get_batch_file_status_dict_format(self) -> None:
        """GetBatchFileStatus returns a dict mapping paths to statuses."""
        paths = [
            os.path.join(self._sync_root, "document.pdf"),
            os.path.join(self._sync_root, "photos/"),
            os.path.join(self._sync_root, "report.docx"),
            os.path.join(self._sync_root, "budget.xlsx"),
        ]
        result = self._files_proxy.call_sync(
            "GetBatchFileStatus",
            GLib.Variant("(as)", (paths,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )

        # Result is (a{ss},) — extract the dict
        dict_variant = result.get_child_value(0)

        # Verify it is a dict (a{ss})
        self.assertTrue(
            dict_variant.get_type_string() == "a{ss}",
            f"Expected a{{ss}} dict, got {dict_variant.get_type_string()}",
        )

        # Unpack and verify contents
        n_entries = dict_variant.n_children()
        self.assertEqual(n_entries, len(paths), f"Expected {len(paths)} entries, got {n_entries}")

        expected = {
            paths[0]: "synced",
            paths[1]: "cloud-only",
            paths[2]: "syncing",
            paths[3]: "conflict",
        }

        for i in range(n_entries):
            entry = dict_variant.get_child_value(i)
            key = entry.get_child_value(0).get_string()
            value = entry.get_child_value(1).get_string()
            self.assertIn(key, expected, f"Unexpected key in batch result: {key}")
            self.assertEqual(
                value, expected[key],
                f"For {key}: expected '{expected[key]}', got '{value}'",
            )

    def test_get_batch_file_status_empty(self) -> None:
        """GetBatchFileStatus with no paths returns an empty dict."""
        result = self._files_proxy.call_sync(
            "GetBatchFileStatus",
            GLib.Variant("(as)", ([],)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        dict_variant = result.get_child_value(0)
        self.assertEqual(dict_variant.n_children(), 0, "Expected empty dict for empty input")

    def test_get_batch_file_status_with_unknown(self) -> None:
        """GetBatchFileStatus includes 'unknown' for unrecognised paths."""
        paths = [
            os.path.join(self._sync_root, "document.pdf"),
            os.path.join(self._sync_root, "totally-made-up.bin"),
        ]
        result = self._files_proxy.call_sync(
            "GetBatchFileStatus",
            GLib.Variant("(as)", (paths,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        dict_variant = result.get_child_value(0)
        statuses = {}
        for i in range(dict_variant.n_children()):
            entry = dict_variant.get_child_value(i)
            statuses[entry.get_child_value(0).get_string()] = entry.get_child_value(1).get_string()

        self.assertEqual(statuses[paths[0]], "synced")
        self.assertEqual(statuses[paths[1]], "unknown")

    # ----- T066: PinFile error case ------------------------------------------

    def test_pin_file_nonexistent_path(self) -> None:
        """PinFile on a non-existent path should not raise a D-Bus error.

        The mock daemon accepts any path for PinFile — it sets the status to
        'synced' regardless.  This tests that the call completes without
        exception (no D-Bus method error), which exercises the error handling
        code path in the Nautilus extension.
        """
        path = os.path.join(self._sync_root, "this-does-not-exist.txt")

        # The mock daemon's PinFile accepts all paths (sets status to 'synced').
        # In production, the real daemon would return an InvalidPath error.
        # Here we verify the mock handles it gracefully (no exception).
        try:
            self._files_proxy.call_sync(
                "PinFile",
                GLib.Variant("(s)", (path,)),
                Gio.DBusCallFlags.NONE,
                5000,
                None,
            )
        except GLib.Error as e:
            # If the mock daemon returns an error, that's also acceptable.
            # We just verify the call didn't crash or hang.
            print(f"PinFile returned error (acceptable): {e.message}")

        # Verify the status was set (mock always pins to 'synced')
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(
            status, "synced",
            f"After PinFile, expected 'synced', got '{status}'",
        )

    # ----- T066: Additional action tests -------------------------------------

    def test_unpin_file(self) -> None:
        """UnpinFile sets a synced file to cloud-only."""
        path = os.path.join(self._sync_root, "notes.txt")

        # Verify initial status is synced
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        self.assertEqual(result.get_child_value(0).get_string(), "synced")

        # Unpin the file
        self._files_proxy.call_sync(
            "UnpinFile",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )

        # Verify new status
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "cloud-only", f"After UnpinFile, expected 'cloud-only', got '{status}'")

    def test_sync_path(self) -> None:
        """SyncPath sets a file status to 'syncing'."""
        path = os.path.join(self._sync_root, "document.pdf")

        self._files_proxy.call_sync(
            "SyncPath",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )

        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "syncing", f"After SyncPath, expected 'syncing', got '{status}'")

    def test_get_conflicts(self) -> None:
        """GetConflicts returns a list of absolute paths with conflict status."""
        result = self._files_proxy.call_sync(
            "GetConflicts",
            None,
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        conflicts_variant = result.get_child_value(0)
        conflicts = [
            conflicts_variant.get_child_value(i).get_string()
            for i in range(conflicts_variant.n_children())
        ]

        # The mock daemon has "budget.xlsx" as conflict
        expected_path = os.path.join(self._sync_root, "budget.xlsx")
        self.assertIn(expected_path, conflicts, f"Expected {expected_path} in conflicts list")


    # ----- Settings interface tests ------------------------------------------

    def test_settings_get_config_returns_yaml(self) -> None:
        """Settings.GetConfig returns a YAML string containing sync_root."""
        result = self._settings_proxy.call_sync(
            "GetConfig",
            None,
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        yaml_str = result.get_child_value(0).get_string()
        self.assertIn("sync_root:", yaml_str, "Config YAML should contain sync_root key")
        self.assertIn(self._sync_root, yaml_str, "Config sync_root should match test sync root")

    def test_settings_get_selected_folders(self) -> None:
        """Settings.GetSelectedFolders returns a non-empty list."""
        result = self._settings_proxy.call_sync(
            "GetSelectedFolders",
            None,
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        folders_variant = result.get_child_value(0)
        count = folders_variant.n_children()
        self.assertGreater(count, 0, "Selected folders list should not be empty")

        # Verify first folder is a string path
        first = folders_variant.get_child_value(0).get_string()
        self.assertTrue(first.startswith("/"), f"Folder path should start with /, got: {first}")

    def test_settings_get_exclusion_patterns(self) -> None:
        """Settings.GetExclusionPatterns returns a list of glob patterns."""
        result = self._settings_proxy.call_sync(
            "GetExclusionPatterns",
            None,
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        patterns_variant = result.get_child_value(0)
        count = patterns_variant.n_children()
        self.assertGreater(count, 0, "Exclusion patterns list should not be empty")

        # Verify *.tmp is in the list (mock default)
        patterns = [
            patterns_variant.get_child_value(i).get_string()
            for i in range(count)
        ]
        self.assertIn("*.tmp", patterns, "Should contain *.tmp exclusion pattern")

    def test_settings_get_remote_folder_tree(self) -> None:
        """Settings.GetRemoteFolderTree returns valid JSON."""
        result = self._settings_proxy.call_sync(
            "GetRemoteFolderTree",
            None,
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        import json

        tree_json = result.get_child_value(0).get_string()
        tree = json.loads(tree_json)
        self.assertEqual(tree["name"], "root", "Root folder name should be 'root'")
        self.assertIn("children", tree, "Root should have children")
        self.assertGreater(len(tree["children"]), 0, "Root should have at least one child")

    # ----- Sync interface tests ----------------------------------------------

    def test_sync_status_property(self) -> None:
        """Sync.SyncStatus property returns a valid status string."""
        status_variant = self._sync_proxy.get_cached_property("SyncStatus")
        self.assertIsNotNone(status_variant, "SyncStatus property should be cached")
        status = status_variant.get_string()
        valid = {"idle", "syncing", "paused", "error", "offline"}
        self.assertIn(status, valid, f"SyncStatus '{status}' not in {valid}")

    def test_sync_pending_changes_property(self) -> None:
        """Sync.PendingChanges property returns a non-negative integer."""
        pending_variant = self._sync_proxy.get_cached_property("PendingChanges")
        self.assertIsNotNone(pending_variant, "PendingChanges property should be cached")
        pending = pending_variant.get_uint32()
        self.assertGreaterEqual(pending, 0, "PendingChanges should be non-negative")

    # ----- Status interface tests --------------------------------------------

    def test_status_connection_status_property(self) -> None:
        """Status.ConnectionStatus property returns a valid status."""
        conn_variant = self._status_proxy.get_cached_property("ConnectionStatus")
        self.assertIsNotNone(conn_variant, "ConnectionStatus property should be cached")
        conn = conn_variant.get_string()
        valid = {"online", "offline", "reconnecting"}
        self.assertIn(conn, valid, f"ConnectionStatus '{conn}' not in {valid}")

    def test_status_get_quota(self) -> None:
        """Status.GetQuota returns (used, total) with total > 0."""
        result = self._status_proxy.call_sync(
            "GetQuota",
            None,
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        used = result.get_child_value(0).get_uint64()
        total = result.get_child_value(1).get_uint64()
        self.assertGreater(total, 0, "Quota total should be > 0")
        self.assertLessEqual(used, total, "Quota used should be <= total")

    # ----- PinFile success case ----------------------------------------------

    def test_pin_file_success(self) -> None:
        """PinFile on a cloud-only file transitions it to synced."""
        path = os.path.join(self._sync_root, "photos/vacation/beach.jpg")

        # Verify initial status is cloud-only
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        self.assertEqual(result.get_child_value(0).get_string(), "cloud-only")

        # Pin the file
        self._files_proxy.call_sync(
            "PinFile",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )

        # Verify new status is synced
        result = self._files_proxy.call_sync(
            "GetFileStatus",
            GLib.Variant("(s)", (path,)),
            Gio.DBusCallFlags.NONE,
            5000,
            None,
        )
        status = result.get_child_value(0).get_string()
        self.assertEqual(status, "synced", f"After PinFile, expected 'synced', got '{status}'")


if __name__ == "__main__":
    unittest.main(verbosity=2)
