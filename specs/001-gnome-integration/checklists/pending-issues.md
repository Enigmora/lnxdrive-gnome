# Pending Issues — GNOME Desktop Integration

**Date**: 2026-02-05 | **Branch**: `001-gnome-integration`
**Source**: `/speckit.analyze` cross-artifact analysis
**Status**: 5 HIGH resolved, 16 MEDIUM/LOW pending for implementation

> These issues are non-blocking and should be resolved **during implementation**
> of their corresponding phase. Each issue is tagged with its target phase
> so `/speckit.tasks` and `/speckit.implement` can reference them.

---

## Resolved (for reference)

| ID | Severity | Resolution |
|----|----------|-----------|
| I1 | HIGH | Added `Unknown` state to FR-001 |
| I4 | HIGH | Added FR Traceability to contracts |
| A1 | HIGH | Linked FR-027 → SC-005 (≤500ms) |
| U1 | HIGH | Created FR-036 + `InsufficientDiskSpace` error |
| U2 | HIGH | Created FR-037 + `FileInUse` error |

---

## Pending by Implementation Phase

### Phase A: Foundation & D-Bus Mock

_No pending issues._

### Phase B: Nautilus Extension

| ID | Sev | Issue | Action Required |
|----|-----|-------|-----------------|
| I2 | MED | FR-004 says "icon for each state" but `Excluded` has no icon in data-model.md | Decide: excluded files show no emblem (current design) or add `lnxdrive-excluded.svg`. Document decision in code comment. |
| I3 | MED | `Unknown` icon exists in plan but `Excluded` does not — intentional asymmetry? | Confirm alignment: `Unknown` = daemon unavailable (needs icon), `Excluded` = user-configured (no icon needed). Add comment in InfoProvider. |
| FR-008 | MED | Plan doesn't mention explicit notifications for context menu actions | Decide in MenuProvider: rely on overlay icon change as visual feedback, or add `GNotification`. Overlay icon change is likely sufficient. |

### Phase C: GNOME Shell Extension

| ID | Sev | Issue | Action Required |
|----|-----|-------|-----------------|
| I5 | MED | Quota format not specified (bytes? percentage? bar?) | Decide during menuItems.js implementation. Recommendation: progress bar + "X.X GB / Y GB" text (matches GNOME Files pattern). |
| A2 | MED | FR-028 "no more resources than typical extension" — no baseline metric | During testing, measure idle RSS and CPU. Document baseline: e.g., <50MB RSS, <1% CPU idle. Add to SC or accept FR-028 as aspirational. |
| G1 | MED | SC-008 (10s reconnection) not explicitly linked to FR-025 | When implementing `notify::g-name-owner` handler, add code comment referencing both FR-025 and SC-008. |

### Phase D: Preferences Panel

| ID | Sev | Issue | Action Required |
|----|-----|-------|-----------------|
| U6 | LOW | `GetRemoteFolderTree()` JSON schema undocumented | Define schema when implementing `folder_tree.rs`. Minimum: `{ "name": str, "path": str, "children": [...] }`. Update contracts after. |
| G3 | LOW | GSettings schema for window state has no FR | Acceptable — implementation detail. No action needed. |

### Phase E: Onboarding Wizard

| ID | Sev | Issue | Action Required |
|----|-----|-------|-----------------|
| U3 | MED | Who handles OAuth2 loopback HTTP server? Daemon or prefs app? | Clarify during `auth_page.rs` implementation. Expected: daemon runs loopback server, emits `AuthStateChanged`; app just listens. Document in code. |
| U4 | MED | `Auth.CompleteAuth()` purpose unclear if daemon auto-completes via loopback | Clarify: `CompleteAuth` = manual flow (CLI, GOA Phase F); loopback flow = daemon auto-completes. Add docstring to contracts after confirming. |

### Phase F: GOA Provider (P3, deferred)

_Issues U3/U4 may surface here too. Resolve in Phase E first._

### Cross-cutting (any phase)

| ID | Sev | Issue | Action Required |
|----|-----|-------|-----------------|
| A3 | MED | FR-029 (HiDPI) not in plan, no test criteria | GTK4/libadwaita + SVG icons handle HiDPI natively. Verify during testing at 1x, 1.5x, 2x. No special code needed. |
| G2 | MED | FR-029 not mentioned in any plan phase | Add brief note when implementing icon installation: "SVG emblems scale natively for HiDPI (FR-029)". |
| A4 | LOW | SC-006 (95% users) not measurable without user testing | Accept as aspirational. Ensure GNOME HIG compliance as proxy. |
| D1 | LOW | FR-006 duplicates US2 scenarios (pin/unpin/sync) | Acceptable redundancy — FR is normative, US is narrative. No action. |
| U5 | LOW | `sync_on_startup` in data-model.md but no FR in spec | If exposed in prefs UI, add FR. If YAML-only, document as out-of-scope for UI. |
| T1 | LOW | Terminology: "solo en la nube" vs "placeholder" vs `CloudOnly` vs `cloud-only` | Add glossary to spec.md or code docs: `CloudOnly` = canonical enum value, "cloud-only" = D-Bus string, "placeholder" = user-facing concept. |
| T2 | LOW | `UnpinFile` vs "dehydrate" — same operation? | Yes: `UnpinFile` = unpin + dehydrate (returns to placeholder). Add docstring to D-Bus client code. |

---

## Summary

| Severity | Count | Blocking? |
|----------|-------|-----------|
| MEDIUM | 8 | No — resolve during implementation |
| LOW | 8 | No — resolve opportunistically |
| **Total pending** | **16** | **None blocking** |

---

*This file is consumed by `/speckit.tasks` and `/speckit.implement` to ensure issues are addressed in context.*
