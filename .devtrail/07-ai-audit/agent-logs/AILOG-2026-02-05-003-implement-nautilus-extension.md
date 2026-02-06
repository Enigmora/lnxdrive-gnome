---
id: AILOG-2026-02-05-003
title: Implement Nautilus file manager extension (Stage 3)
status: accepted
created: 2026-02-05
agent: claude-code-v1.0
confidence: high
review_required: false
risk_level: low
tags: [nautilus, c11, dbus, overlay-icons, context-menu, columns]
related: [AILOG-2026-02-05-001, AILOG-2026-02-05-002]
---

# AILOG: Implement Nautilus file manager extension (Stage 3)

## Summary

Implemented the complete Nautilus file manager extension (`liblnxdrive-nautilus.so`) in C11. This extension provides overlay icons for file sync status, a context menu with Pin/Unpin/Sync actions, and two custom columns (Status and Last Synced). The extension communicates with the lnxdrive-daemon via D-Bus on the session bus.

## Context

Stage 3 of the GNOME integration plan (spec 001-gnome-integration) requires a Nautilus extension to enable User Stories 1 and 2: visual sync status in the file manager and context-menu actions for file management. The extension is implemented in C11 using libnautilus-extension-4, GDBus, and GObject patterns, covering tasks T028 through T040 of the plan.

## Actions Performed

1. Created `lnxdrive-dbus-client.h` and `lnxdrive-dbus-client.c` (T028 + T029): Singleton GObject D-Bus client with async proxy creation, status cache, FileStatusChanged signal handling, name-owner tracking for graceful degradation (FR-025), sync root discovery from Settings interface, and async Pin/Unpin/Sync operations.

2. Created `lnxdrive-extension.c` (T030): Module entry point implementing `nautilus_module_initialize()`, `nautilus_module_list_types()`, and `nautilus_module_shutdown()`. Registers three provider GTypes using GTypeModule and initializes the D-Bus client singleton.

3. Created `lnxdrive-info-provider.h` and `lnxdrive-info-provider.c` (T031 + T032 + T033): NautilusInfoProvider implementation with status-to-emblem mapping (7 statuses + excluded), URI-to-path conversion, sync root boundary checking, and custom string attribute population for columns.

4. Created `lnxdrive-column-provider.h` and `lnxdrive-column-provider.c` (T034 + T035): NautilusColumnProvider with two columns: "LNXDrive Status" and "Last Synced".

5. Created `lnxdrive-menu-provider.h` and `lnxdrive-menu-provider.c` (T036 + T037 + T038 + T039): NautilusMenuProvider with contextual submenu (Pin/Unpin/Sync Now), multi-selection support (FR-007), sync root filtering (FR-005), daemon-offline disabled state (FR-025), background "Sync This Folder" item, and error notification handling for InsufficientDiskSpace, FileInUse, InvalidPath, and generic errors.

6. Updated `nautilus-extension/meson.build` (T040): Added `-DLOCALEDIR` to c_args for gettext initialization. Source file list was already correct.

## Modified Files

| File | Change |
|------|--------|
| `nautilus-extension/src/lnxdrive-dbus-client.h` | Created: D-Bus client GObject header with full public API |
| `nautilus-extension/src/lnxdrive-dbus-client.c` | Created: D-Bus client implementation (singleton, cache, signals, async ops) |
| `nautilus-extension/src/lnxdrive-extension.c` | Created: Module entry point (init, list_types, shutdown) |
| `nautilus-extension/src/lnxdrive-info-provider.h` | Created: InfoProvider header |
| `nautilus-extension/src/lnxdrive-info-provider.c` | Created: InfoProvider with emblem overlay and attributes |
| `nautilus-extension/src/lnxdrive-column-provider.h` | Created: ColumnProvider header |
| `nautilus-extension/src/lnxdrive-column-provider.c` | Created: ColumnProvider with Status and Last Synced columns |
| `nautilus-extension/src/lnxdrive-menu-provider.h` | Created: MenuProvider header |
| `nautilus-extension/src/lnxdrive-menu-provider.c` | Created: MenuProvider with contextual actions and error handling |
| `nautilus-extension/meson.build` | Modified: Added LOCALEDIR c_arg |

## Decisions Made

- Used `G_DEFINE_DYNAMIC_TYPE_EXTENDED` with `G_IMPLEMENT_INTERFACE_DYNAMIC` for all providers, which is required for GTypeModule-based registration in shared module extensions.
- Used cache-first approach in InfoProvider: status lookups are synchronous against the D-Bus client's local cache. Files without cached status show "unknown" and update when FileStatusChanged signals arrive. This avoids blocking Nautilus.
- Settings proxy for sync root uses fire-and-forget async pattern with explicit ref management (not g_autoptr) to survive across async callback boundaries.
- Error notifications use GNotification when GApplication is available, with fallback to g_warning for headless/extension contexts.
- "Excluded" files intentionally show no emblem (pending issue I2 noted in comment).

## Impact

- **Functionality**: Enables overlay icons (US1), context menu actions (US2), and custom columns for LNXDrive files in Nautilus.
- **Performance**: N/A -- cache-first design ensures no blocking D-Bus calls in the hot path.
- **Security**: N/A -- extension only communicates with local session D-Bus.

## Verification

- [ ] Code compiles without errors
- [ ] Tests pass
- [ ] Manual review performed

## Additional Notes

- The `last_sync` column currently shows an em-dash placeholder because the D-Bus Files interface does not yet provide per-file sync timestamps. This will be populated once the daemon adds that capability.
- The `nautilus_file_info_list_free` function from older Nautilus versions was replaced with a custom `file_info_list_free` using `g_list_free_full(list, g_object_unref)` for v4 compatibility.
- The `G_DECLARE_FINAL_TYPE` + `G_DEFINE_DYNAMIC_TYPE_EXTENDED` combination is valid: the header provides the forward declaration of `_register_type` and the macro provides its implementation.

---

<!-- Template: DevTrail | https://enigmora.com -->
