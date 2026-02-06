# D-Bus Contracts: GNOME Integration

**Branch**: `001-gnome-integration` | **Date**: 2026-02-05

This document defines the D-Bus interfaces consumed by the GNOME integration components. These interfaces are provided by `lnxdrive-daemon` (defined in `org.enigmora.LNXDrive`). The GNOME components are **clients only** — they do not serve D-Bus interfaces.

---

## Consumer Map

| GNOME Component | D-Bus Interfaces Consumed |
|-----------------|---------------------------|
| Nautilus extension (C) | `.Files`, `.Sync` (signals only) |
| GNOME Shell extension (GJS) | `.Sync`, `.Status`, `.Manager` |
| Preferences panel (Rust) | `.Settings`, `.Sync`, `.Files`, `.Status` |
| Onboarding wizard (Rust) | `.Manager`, `.Auth` |

---

## Interface: org.enigmora.LNXDrive.Files

Used primarily by the Nautilus extension for per-file status queries and actions.

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `GetFileStatus(path: s) → (status: s)` | `in:s out:s` | Returns overlay status for a file path |
| `GetBatchFileStatus(paths: as) → (statuses: a{ss})` | `in:as out:a{ss}` | Batch query for multiple files (performance) |
| `PinFile(path: s)` | `in:s` | Make file available offline (hydrate + pin) |
| `UnpinFile(path: s)` | `in:s` | Free space (dehydrate) |
| `SyncPath(path: s)` | `in:s` | Force immediate sync of a path |
| `GetConflicts() → (paths: as)` | `out:as` | List all conflicted file paths |

### Signals

| Signal | Signature | Description |
|--------|-----------|-------------|
| `FileStatusChanged(path: s, status: s)` | `ss` | Emitted when a file's sync status changes |

### Notes
- `GetBatchFileStatus` is critical for Nautilus performance — the extension should batch-query visible files rather than making individual calls.
- Status values match `OverlayStatus` enum: `synced`, `cloud-only`, `syncing`, `pending`, `conflict`, `error`, `excluded`. When the daemon is unavailable, clients locally derive the `unknown` state (not returned by D-Bus).

### FR Traceability
- `GetFileStatus` / `GetBatchFileStatus` → FR-001, FR-002, FR-027 (overlay icons, real-time, performance)
- `PinFile` → FR-006 ("Mantener disponible offline"), FR-036 (disk space check)
- `UnpinFile` → FR-006 ("Liberar espacio" / dehydrate), FR-037 (file-in-use check)
- `SyncPath` → FR-006 ("Sincronizar ahora")
- `GetConflicts` → FR-010 (conflict count in indicator)
- `FileStatusChanged` → FR-002, FR-026 (real-time signal updates)

---

## Interface: org.enigmora.LNXDrive.Sync

Used by Shell extension for global sync control and by Nautilus for sync signals.

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `SyncNow()` | (none) | Trigger immediate full sync |
| `Pause()` | (none) | Pause sync |
| `Resume()` | (none) | Resume sync |

### Properties

| Property | Type | Access | Description |
|----------|------|--------|-------------|
| `SyncStatus` | `s` | read | Global state: `idle`, `syncing`, `paused`, `error` |
| `LastSyncTime` | `x` | read | Unix timestamp of last complete sync |
| `PendingChanges` | `u` | read | Number of pending file operations |

### Signals

| Signal | Signature | Description |
|--------|-----------|-------------|
| `SyncStarted()` | (none) | Sync cycle began |
| `SyncCompleted(files_synced: u, errors: u)` | `uu` | Sync cycle completed |
| `SyncProgress(file: s, current: u, total: u)` | `suu` | Per-file progress |
| `ConflictDetected(path: s, type: s)` | `ss` | New conflict |

### FR Traceability
- `SyncNow` / `Pause` / `Resume` → FR-011 (quick actions in indicator)
- `SyncStatus` property → FR-009 (indicator icon state)
- `PendingChanges` property → FR-010 (pending files count in indicator menu)
- `SyncProgress` signal → FR-010 (sync progress display)
- `ConflictDetected` signal → FR-010 (conflict count)

---

## Interface: org.enigmora.LNXDrive.Status

Used by Shell extension for account and quota information.

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `GetQuota() → (used: t, total: t)` | `out:t out:t` | Storage quota in bytes |
| `GetAccountInfo() → (info: a{sv})` | `out:a{sv}` | Account details dict |

### Properties

| Property | Type | Access | Description |
|----------|------|--------|-------------|
| `ConnectionStatus` | `s` | read | `online`, `offline`, `reconnecting` |

### Signals

| Signal | Signature | Description |
|--------|-----------|-------------|
| `QuotaChanged(used: t, total: t)` | `tt` | Quota update |
| `ConnectionChanged(status: s)` | `s` | Network state change |

---

## Interface: org.enigmora.LNXDrive.Manager

Used by Shell extension and onboarding wizard for daemon lifecycle.

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `Start()` | (none) | Start the daemon |
| `Stop()` | (none) | Stop the daemon |
| `Restart()` | (none) | Restart the daemon |
| `GetStatus() → (status: s)` | `out:s` | Daemon status |

### Properties

| Property | Type | Access | Description |
|----------|------|--------|-------------|
| `Version` | `s` | read | Daemon version string |
| `IsRunning` | `b` | read | Whether daemon is active |

---

## Interface: org.enigmora.LNXDrive.Settings (consumed by Preferences panel)

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `GetConfig() → (yaml: s)` | `out:s` | Full config as YAML string |
| `SetConfig(yaml: s)` | `in:s` | Apply full config (validates before applying) |
| `GetSelectedFolders() → (folders: as)` | `out:as` | Currently synced folders |
| `SetSelectedFolders(folders: as)` | `in:as` | Update selective sync folders |
| `GetExclusionPatterns() → (patterns: as)` | `out:as` | Current exclusion patterns |
| `SetExclusionPatterns(patterns: as)` | `in:as` | Update exclusion patterns |
| `GetRemoteFolderTree() → (tree: s)` | `out:s` | JSON tree of remote folders for selective sync UI |

### Signals

| Signal | Signature | Description |
|--------|-----------|-------------|
| `ConfigChanged(key: s)` | `s` | Emitted when any config value changes |

---

## Interface: org.enigmora.LNXDrive.Auth (consumed by Onboarding wizard)

### Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `StartAuth() → (auth_url: s, state: s)` | `out:s out:s` | Generate OAuth2 URL + CSRF state |
| `CompleteAuth(code: s, state: s) → (success: b)` | `in:s in:s out:b` | Exchange code for tokens |
| `IsAuthenticated() → (authenticated: b)` | `out:b` | Check if account is configured |
| `Logout()` | (none) | Remove account and tokens |

### Signals

| Signal | Signature | Description |
|--------|-----------|-------------|
| `AuthStateChanged(state: s)` | `s` | `authenticated`, `expired`, `disconnected` |

---

## Error Handling Convention

All D-Bus method calls may return standard D-Bus errors:

| Error Name | When |
|------------|------|
| `org.enigmora.LNXDrive.Error.NotRunning` | Daemon not active |
| `org.enigmora.LNXDrive.Error.NotAuthenticated` | No account configured |
| `org.enigmora.LNXDrive.Error.InvalidPath` | Path not in sync root |
| `org.enigmora.LNXDrive.Error.InvalidConfig` | Config validation failed |
| `org.enigmora.LNXDrive.Error.NetworkError` | Cannot reach cloud |
| `org.enigmora.LNXDrive.Error.InsufficientDiskSpace` | Not enough local disk space to hydrate file (FR-036) |
| `org.enigmora.LNXDrive.Error.FileInUse` | File is actively used by another process, cannot dehydrate (FR-037) |

GNOME components must handle these errors gracefully and display user-friendly messages.
