# Data Model: GNOME Desktop Integration

**Branch**: `001-gnome-integration` | **Date**: 2026-02-05

---

## Entities

### FileOverlayState

Represents the visual state of a file in Nautilus, derived from the daemon's sync state.

| Field | Type | Description |
|-------|------|-------------|
| `uri` | String | File URI (e.g., `file:///home/user/OneDrive/doc.pdf`) |
| `status` | OverlayStatus (enum) | Visual status category |
| `last_sync` | Timestamp (optional) | Last successful sync time |
| `error_message` | String (optional) | Human-readable error description |

**OverlayStatus enum values:**

| Value | Emblem Icon | Description |
|-------|-------------|-------------|
| `Synced` | `lnxdrive-synced` | Synchronized, available locally |
| `CloudOnly` | `lnxdrive-cloud-only` | Placeholder, content in cloud |
| `Syncing` | `lnxdrive-syncing` | Currently uploading/downloading |
| `Pending` | `lnxdrive-pending` | Queued for sync |
| `Conflict` | `lnxdrive-conflict` | Sync conflict detected |
| `Error` | `lnxdrive-error` | Sync error |
| `Excluded` | (none) | Excluded by pattern/config |
| `Unknown` | `lnxdrive-unknown` | Daemon unavailable |

**State Transitions:**

```
                    ┌──────────────────────────────────────┐
                    │                                      │
                    ▼                                      │
CloudOnly ──► Syncing ──► Synced ──► Pending ──► Syncing ──┘
    ▲             │          │          │
    │             ▼          │          ▼
    │          Error         │       Conflict
    │             │          │          │
    │             ▼          │          ▼
    └─────── (retry) ◄──────┘     (resolved)
```

- `CloudOnly → Syncing`: User requests pin / file accessed
- `Syncing → Synced`: Download/upload completed
- `Syncing → Error`: Transfer failed
- `Synced → Pending`: Local modification detected
- `Pending → Syncing`: Sync engine picks up change
- `Syncing → Conflict`: Remote version differs
- `Conflict → Synced`: Conflict resolved
- `Synced → CloudOnly`: User frees space (dehydrate)

---

### SyncStatusSummary

Global sync state consumed by the GNOME Shell indicator.

| Field | Type | Description |
|-------|------|-------------|
| `global_state` | GlobalSyncState (enum) | Overall daemon state |
| `current_file` | String (optional) | File currently being synced |
| `progress_percent` | u8 (0-100) | Progress of current file |
| `files_pending` | u32 | Files queued for sync |
| `files_syncing` | u32 | Files currently transferring |
| `conflicts_count` | u32 | Unresolved conflicts |
| `quota_used` | u64 | Bytes used on cloud |
| `quota_total` | u64 | Total bytes available |
| `connection_status` | ConnectionStatus (enum) | Network state |

**GlobalSyncState enum:**

| Value | Icon State | Description |
|-------|-----------|-------------|
| `Idle` | Static cloud icon | Nothing to sync |
| `Syncing` | Animated sync icon | Active transfer |
| `Paused` | Paused icon | User paused |
| `Error` | Error badge | One or more errors |
| `Offline` | Disconnected icon | No network |

**ConnectionStatus enum:** `Online`, `Offline`, `Reconnecting`

---

### UserPreferences

Configuration managed by the preferences panel, backed by `~/.config/lnxdrive/config.yaml`.

| Field | Type | Description |
|-------|------|-------------|
| `sync_root` | Path | Local sync root directory |
| `sync_mode` | SyncMode (enum) | `Automatic` or `Manual` |
| `selected_folders` | Vec<Path> | Folders to sync (selective sync) |
| `exclusion_patterns` | Vec<String> | Glob patterns to exclude |
| `conflict_policy` | ConflictPolicy (enum) | Default conflict resolution |
| `bandwidth_limit_up` | u32 (optional) | Upload limit in KB/s (0 = unlimited) |
| `bandwidth_limit_down` | u32 (optional) | Download limit in KB/s (0 = unlimited) |
| `sync_on_startup` | bool | Auto-start sync on login |

**ConflictPolicy enum:** `AlwaysAsk`, `KeepLocal`, `KeepRemote`, `KeepBoth`

**SyncMode enum:** `Automatic`, `Manual`

---

### AccountConnection

Relationship between an authenticated account and LNXDrive sync configuration.

| Field | Type | Description |
|-------|------|-------------|
| `account_id` | String | Unique account identifier |
| `display_name` | String | User display name |
| `email` | String | Account email |
| `auth_state` | AuthState (enum) | Current authentication state |
| `provider` | String | Cloud provider (e.g., `onedrive`) |

**AuthState enum:** `Authenticated`, `TokenExpired`, `Disconnected`, `Pending`

**Lifecycle:**

```
Disconnected ──► Pending ──► Authenticated
                    │              │
                    ▼              ▼
                 (cancel)    TokenExpired
                    │              │
                    ▼              ▼
              Disconnected   (auto-refresh)
                                   │
                                   ▼
                             Authenticated
```

---

### OnboardingState

Tracks the first-run wizard progress (transient, not persisted until completion).

| Field | Type | Description |
|-------|------|-------------|
| `current_step` | OnboardingStep (enum) | Current wizard step |
| `account` | AccountConnection (optional) | Account if auth completed |
| `sync_root` | Path (optional) | Selected folder if chosen |

**OnboardingStep enum:** `Welcome`, `Authentication`, `FolderSelection`, `Confirmation`, `Complete`

**Rules:**
- If user cancels, all transient state is discarded.
- Only persisted to `config.yaml` upon `Complete` step.
- On next launch, if no config exists, wizard restarts from `Welcome`.

---

## Relationships

```
AccountConnection 1 ──── 1 UserPreferences
        │
        │ authenticated by
        ▼
OnboardingState (transient, creates both)

UserPreferences 1 ──── * FileOverlayState
        │                     │
        │ selected_folders    │ filtered by exclusion_patterns
        │                     │
        └─────────────────────┘

SyncStatusSummary ◄──── aggregates ──── * FileOverlayState
```
