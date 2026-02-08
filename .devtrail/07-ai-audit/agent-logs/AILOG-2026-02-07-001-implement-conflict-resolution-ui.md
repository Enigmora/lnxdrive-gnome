---
id: AILOG-2026-02-07-001
title: Implement conflict resolution UI for GNOME desktop integration
status: accepted
created: 2026-02-07
agent: claude-code-v1.0
confidence: high
review_required: false
risk_level: low
tags: [conflicts, gtk4, dbus, shell-extension, preferences]
related: [AILOG-2026-02-05-005-implement-preferences-panel]
---

# AILOG: Implement conflict resolution UI for GNOME desktop integration

## Summary

Added the complete conflict resolution user interface across all GNOME components: a Preferences dialog for detailed inspection and resolution, per-conflict entries in the shell extension indicator menu, D-Bus client proxy for the Conflicts interface, and updated mock daemon and tests.

## Context

The lnxdrive daemon (Fase 5) now detects and stores conflicts via `com.enigmora.LNXDrive.Conflicts` D-Bus interface. The GNOME integration needed UI components for users to view, inspect, and resolve file conflicts. The preferences app had 3 tabs (Account, Sync, Advanced) but no conflict management. The shell extension showed a simple conflict counter but no individual entries.

## Actions Performed

1. **D-Bus client extension**: Added `LnxdriveConflicts` proxy trait with `list`, `get_details`, `resolve`, `resolve_all` methods and `ConflictDetected`/`ConflictResolved` signals. Added 4 high-level methods to `DbusClient`.
2. **ConflictDetailDialog** (`adw::Dialog`): Side-by-side comparison of local vs remote version metadata (hash, size, modified date), with 3 resolution action rows (Keep Local, Keep Remote, Keep Both). Resolves via D-Bus and closes on success.
3. **ConflictListPage** (`adw::PreferencesPage`): Lists all unresolved conflicts with warning icons. Click opens detail dialog. "Resolve All" button shows strategy chooser (`adw::AlertDialog`). Integrated as 4th tab in `PreferencesDialog`.
4. **ConflictInfo struct**: Lightweight deserialization from daemon JSON with helper methods (`filename()`, `extension()`, `from_json_array()`).
5. **Shell extension dbus.js**: Added `ConflictsInterfaceXml` with full introspection and `ConflictsProxy` in `createProxies()`. Return object now includes `conflicts` proxy.
6. **Shell extension menuItems.js**: Replaced simple counter with per-conflict entries (up to 5 visible + "View all..." link). Listens to both `ConflictDetected` (from Conflicts interface) and legacy signal (from Sync interface). `ConflictResolved` signal removes entries in real-time.
7. **Mock daemon**: Added `ConflictsInterface` (7th interface) with `List`, `GetDetails`, `Resolve`, `ResolveAll` methods, `ConflictDetected`/`ConflictResolved` signals, and 2 pre-populated mock conflicts (`budget.xlsx`, `shared/team-notes.docx`).
8. **Tests**: Added 4 new shell extension tests — proxy existence, `List` method call, `Resolve` method call, `ConflictDetected` signal subscription. All 13 tests pass.

## Modified Files

| File | Change |
|------|--------|
| `preferences/src/conflicts/mod.rs` | **New** — Module declarations |
| `preferences/src/conflicts/conflict_dialog.rs` | **New** — ConflictDetailDialog with side-by-side view + resolution actions |
| `preferences/src/conflicts/conflict_list.rs` | **New** — ConflictListPage with list + Resolve All |
| `preferences/src/dbus_client.rs` | LnxdriveConflicts proxy trait + 4 DbusClient methods |
| `preferences/src/main.rs` | Added `mod conflicts` |
| `preferences/src/preferences/mod.rs` | Added ConflictListPage as 4th tab |
| `shell-extension/.../dbus.js` | ConflictsInterfaceXml + ConflictsProxy in createProxies() |
| `shell-extension/.../menuItems.js` | Per-conflict menu entries with real-time signal updates |
| `tests/mock-dbus-daemon.py` | ConflictsInterface with 4 methods, 2 signals, 2 mock conflicts |
| `tests/test-shell-extension.js` | 4 new conflict-related tests |

## Decisions Made

- **ConflictListPage as separate PreferencesPage** (4th tab) rather than embedding in sync_page.rs. This keeps the sync page focused on settings and gives conflicts their own dedicated space with room for future features (rules management, diff tool launch).
- **Per-conflict entries capped at 5** in the shell indicator to avoid overwhelming the dropdown menu. "View all..." opens the Preferences app.
- **Legacy ConflictDetected signal on Sync interface** still listened to for backwards compatibility with daemon versions that emit on the Sync interface.
- **Resolve returns bool, resolve_all returns u32** — matching the real daemon D-Bus interface signature.

## Impact

- **Functionality**: Users can now view, inspect, and resolve file conflicts from both the GNOME Shell indicator and the Preferences application. Batch resolution available via "Resolve All".
- **Performance**: N/A — UI changes only, no sync path impact.
- **Security**: N/A

## Verification

- [x] Code compiles without errors (`cargo check` clean)
- [x] 13/13 shell extension tests pass (4 new)
- [x] 12/12 Nautilus extension tests pass (unchanged)
- [ ] Manual review performed

## Additional Notes

This commit is on branch `feat/002-conflict-resolution-ui` based on `main`. PR: https://github.com/Enigmora/lnxdrive-gnome/pull/3

The conflict list page rebuilds the `PreferencesGroup` on each refresh rather than diffing individual rows. This is acceptable for the expected conflict count (<100) but could be optimized with a `gio::ListStore` model if needed in the future.

---

<!-- Template: DevTrail | https://enigmora.com -->
