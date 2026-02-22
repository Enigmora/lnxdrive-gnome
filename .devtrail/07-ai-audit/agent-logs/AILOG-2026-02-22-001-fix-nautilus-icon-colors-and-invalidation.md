---
id: AILOG-2026-02-22-001
title: Fix Nautilus extension icon colors and file invalidation
status: accepted
created: 2026-02-22
agent: claude-code-v1.0
confidence: high
review_required: false
risk_level: low
tags: [nautilus, icons, dbus, invalidation]
related: [AILOG-2026-02-05-003]
---

# AILOG: Fix Nautilus extension icon colors and file invalidation

## Summary

Fixed two issues discovered during interactive VM testing: SVG emblem icons
rendered as monochrome (unrecognizable) because they used `fill="currentColor"`,
and the "Sync Now" action appeared to do nothing because Nautilus never
refreshed emblems after D-Bus status changes.

## Context

During VM testing, the Nautilus extension displayed all emblem icons in a single
color (matching the text foreground), making it impossible to distinguish sync
states visually. Additionally, triggering "Sync Now" from the context menu
successfully called `SyncPath` over D-Bus, but Nautilus did not update the
emblem because the invalidation callback in `lnxdrive-extension.c` was a no-op.

## Actions Performed

1. Replaced `fill="currentColor"` with GNOME Adwaita palette colors in all 7 SVG
   emblem icons (green for synced, blue for cloud-only/syncing, amber for
   pending/conflict, red for error, gray for unknown)
2. Added a `tracked_files` hash table in `lnxdrive-info-provider.c` that maps
   file paths to `NautilusFileInfo*` references
3. Connected a signal handler to the D-Bus client's `file-status-changed` GObject
   signal that calls `nautilus_file_info_invalidate_extension_info()` for the
   affected file
4. Removed the no-op `on_invalidate_request` callback and the
   `set_invalidate_func()` call from `lnxdrive-extension.c`

## Modified Files

| File | Change |
|------|--------|
| `nautilus-extension/icons/lnxdrive-synced.svg` | `fill="currentColor"` -> `fill="#2ec27e"` (green) |
| `nautilus-extension/icons/lnxdrive-cloud-only.svg` | `fill="currentColor"` -> `fill="#3584e4"` (blue) |
| `nautilus-extension/icons/lnxdrive-syncing.svg` | `fill="currentColor"` -> `fill="#1c71d8"` (blue) |
| `nautilus-extension/icons/lnxdrive-pending.svg` | `fill="currentColor"` -> `fill="#e5a50a"` (amber) |
| `nautilus-extension/icons/lnxdrive-conflict.svg` | `fill="currentColor"` -> `fill="#e5a50a"` (amber) |
| `nautilus-extension/icons/lnxdrive-error.svg` | `fill="currentColor"` -> `fill="#e01b24"` (red) |
| `nautilus-extension/icons/lnxdrive-unknown.svg` | `fill="currentColor"` -> `fill="#77767b"` (gray) |
| `nautilus-extension/src/lnxdrive-info-provider.c` | Added tracked_files hash table, signal handler, lazy init |
| `nautilus-extension/src/lnxdrive-extension.c` | Removed no-op invalidation callback and set_invalidate_func call |

## Decisions Made

- Used GNOME Adwaita named colors for consistency with the desktop theme
- Chose lazy initialization (on first `update_file_info()` call) rather than
  eager init, to keep the entry point module simple
- Did not modify `lnxdrive-dbus-client.c` — its existing `file-status-changed`
  GObject signal was already sufficient

## Impact

- **Functionality**: Emblem icons now show distinct colors per sync state; Nautilus
  refreshes emblems in real-time when the daemon reports status changes
- **Performance**: Minimal — one hash table lookup per `FileStatusChanged` signal
- **Security**: N/A

## Verification

- [x] Code compiles without errors
- [x] Tests pass (9/9 container tests passed)
- [ ] Manual VM verification pending (rebuild + restart Nautilus)

---

<!-- Template: DevTrail | https://enigmora.com -->
