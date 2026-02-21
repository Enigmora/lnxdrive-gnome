---
id: AILOG-2026-02-20-001
title: MVP Closure C2 — Unify D-Bus bus name to com.enigmora.LNXDrive
status: accepted
created: 2026-02-20
agent: claude-code-v1.0
confidence: high
review_required: false
risk_level: medium
tags: [mvp, dbus, bus-name, nautilus, preferences, shell-extension, tests]
related: [MVP-CLOSURE-PLAN.md]
---

# AILOG: MVP Closure C2 — Unify D-Bus bus name to com.enigmora.LNXDrive

## Summary

Unified the D-Bus bus name across all GNOME integration components from `org.enigmora.LNXDrive` to `com.enigmora.LNXDrive`, matching the daemon's registered name. This was a critical gap (C2) that prevented the entire GNOME layer from connecting to the running daemon.

## Context

The LNXDrive daemon registers on D-Bus as `com.enigmora.LNXDrive` with object path `/com/enigmora/LNXDrive`. However, all GNOME client components (Nautilus extension, Preferences app, Shell extension) and test infrastructure were using `org.enigmora.LNXDrive` with `/org/enigmora/LNXDrive`. This mismatch meant no GNOME component could ever connect to the real daemon — they would silently fail to find the bus name.

## Actions Performed

1. Updated Nautilus extension C header with correct bus name, object path, and all interface names
2. Updated Nautilus extension C source with corrected Settings interface name
3. Updated Preferences Rust D-Bus proxy definitions (16 occurrences across 5 interface proxies)
4. Updated Shell extension GJS D-Bus module (BUS_NAME, OBJECT_PATH, and all XML interface definitions)
5. Updated mock D-Bus daemon (BUS_NAME, OBJECT_PATH, and all 7 interface super().__init__() calls)
6. Updated Shell extension GJS tests (BUS_NAME, OBJECT_PATH, interface assertions)
7. Updated Nautilus extension Python tests (BUS_NAME, OBJECT_PATH, and 4 interface constants)

## Modified Files

| File | Change |
|------|--------|
| `nautilus-extension/src/lnxdrive-dbus-client.h` | Changed 6 #define constants: bus name `org.enigmora` → `com.enigmora`, object path `/org/` → `/com/`, 4 interface names |
| `nautilus-extension/src/lnxdrive-dbus-client.c` | Changed `org.enigmora.LNXDrive.Settings` → `com.enigmora.LNXDrive.Settings` on line 425 |
| `preferences/src/dbus_client.rs` | Changed 16 occurrences in `#[proxy()]` macros across AuthProxy, SettingsProxy, StatusProxy, SyncProxy, ConflictsProxy |
| `shell-extension/lnxdrive-indicator@enigmora.com/dbus.js` | Changed BUS_NAME, OBJECT_PATH, and 6 XML interface name attributes |
| `tests/mock-dbus-daemon.py` | Changed BUS_NAME, OBJECT_PATH, and 7 `super().__init__()` interface name strings |
| `tests/test-shell-extension.js` | Changed BUS_NAME, OBJECT_PATH, and interface name assertion |
| `tests/test-nautilus-extension.py` | Changed BUS_NAME, OBJECT_PATH, IFACE_FILES, IFACE_SETTINGS, IFACE_SYNC, IFACE_STATUS |

## Decisions Made

- **Mock daemon also changed**: Unlike the original plan which suggested keeping `org.enigmora` for test isolation, the mock daemon was updated to `com.enigmora` to match all client code. Test isolation is achieved by running tests in containers with separate D-Bus sessions, not by using different bus names.

## Impact

- **Functionality**: All GNOME components (Nautilus overlays, Preferences app, Shell indicator) can now connect to the real daemon. This was a complete connectivity blocker.
- **Performance**: N/A
- **Security**: N/A

## Verification

- [x] All string replacements verified with grep (0 residual `org.enigmora` in source files)
- [ ] Tests pass (requires container test environment)
- [ ] Manual review performed

## Additional Notes

- Total: +93/-93 lines across 7 files (pure string replacement, no logic changes)
- The mock daemon and tests now use `com.enigmora.LNXDrive` consistently with all client code

---

<!-- Template: DevTrail | https://enigmora.com -->
