# GNOME Online Accounts Provider — LNXDrive

**Status**: P3 — Not yet implemented (deferred)

## Planned Architecture

The GOA provider will be a C shared library implementing the `GoaProvider` GObject
interface, registered with GNOME Online Accounts to provide native Microsoft account
integration in GNOME Settings → Online Accounts.

### Components

| Component | Description |
|-----------|-------------|
| `GoaProvider` implementation | C shared library loaded by `gnome-online-accounts` |
| OAuth2 with WebKitGTK | Embedded web view for the Microsoft authentication flow |
| Token handoff | Pass OAuth2 tokens to `lnxdrive-daemon` via `Auth.CompleteAuth()` D-Bus method |
| Account lifecycle | Monitor GOA for account addition/removal, notify daemon accordingly |

### Functional Requirements Coverage

| FR | Description | Status |
|----|-------------|--------|
| FR-019 | Provider registration in GNOME Online Accounts | Not implemented |
| FR-020 | OAuth2 PKCE authentication via embedded WebView | Not implemented |
| FR-021 | SSO — reuse existing Microsoft accounts from GOA | Not implemented |
| FR-022 | Automatic token refresh via GOA infrastructure | Not implemented |
| FR-023 | Account removal propagation to daemon | Not implemented |

### Why Deferred

The onboarding wizard (US5, P1) provides a fully functional independent authentication
path using system browser + loopback redirect (RFC 8252). GOA integration adds SSO
polish (reuse existing Microsoft accounts) but requires significant platform-specific
plumbing including WebKitGTK embedded views and GOA provider C API integration.

### Dependencies

- `gnome-online-accounts` (libgoa-1.0, libgoa-backend-1.0)
- `webkit6` (WebKitGTK 6.x for embedded OAuth2 view)
- `lnxdrive-daemon` D-Bus interface `org.enigmora.LNXDrive.Auth`

### Build

This component is gated behind the `enable_goa` Meson option (default: false):

```bash
meson setup builddir -Denable_goa=true
```

---

*LNXDrive GNOME Integration — [Enigmora](https://enigmora.com)*
