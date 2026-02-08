// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Enigmora <https://enigmora.com>

/**
 * LNXDrive Menu Items
 *
 * Builds the dropdown menu for the panel indicator, including:
 * - Sync progress section (current file, percentage, pending count)
 * - Conflicts section (conflict count updated via signals)
 * - Quota section (used/total with visual progress bar)
 * - Actions section (Pause/Resume, Sync Now, Preferences)
 *
 * Implements: FR-010, FR-011, FR-026
 */

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import St from 'gi://St';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

/**
 * Format a byte count into a human-readable string (e.g., "4.2 GB").
 *
 * @param {number} bytes - The byte count to format.
 * @returns {string} Human-readable size string.
 */
function _formatBytes(bytes) {
    if (bytes === 0)
        return '0 B';

    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    const k = 1024;
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    const value = bytes / Math.pow(k, i);

    return `${value.toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

/**
 * Format a Unix timestamp into a human-readable relative time string.
 *
 * @param {number} timestamp - Unix timestamp in seconds (0 = never).
 * @param {function(string): string} _ - Gettext function.
 * @returns {string} Relative time string (e.g., "5 min ago", "2 hours ago").
 */
function _formatLastSyncTime(timestamp, _) {
    if (timestamp === 0)
        return _('Never synced');

    const now = Math.floor(GLib.get_real_time() / 1000000);
    const diff = now - timestamp;

    if (diff < 60)
        return _('Just now');
    if (diff < 3600) {
        const mins = Math.floor(diff / 60);
        return `${mins} ${mins !== 1 ? _('min ago') : _('min ago')}`;
    }
    if (diff < 86400) {
        const hours = Math.floor(diff / 3600);
        return `${hours} ${hours !== 1 ? _('hours ago') : _('hour ago')}`;
    }
    const days = Math.floor(diff / 86400);
    return `${days} ${days !== 1 ? _('days ago') : _('day ago')}`;
}

/**
 * Build the complete dropdown menu for the LNXDrive indicator.
 *
 * @param {PopupMenu.PopupMenu} menu - The indicator's popup menu to populate.
 * @param {{sync: Gio.DBusProxy, status: Gio.DBusProxy, manager: Gio.DBusProxy}} proxies
 *   The D-Bus proxy objects for communicating with the daemon.
 * @param {function(string): string} [gettext] - Gettext function for i18n.
 *   If not provided, strings are returned as-is (no translation).
 * @returns {Array<{proxy: Gio.DBusProxy, id: number}>}
 *   Array of signal connection records for cleanup by the indicator.
 */
export function buildMenu(menu, proxies, gettext) {
    const _ = gettext || (s => s);
    const signalIds = [];

    menu.removeAll();

    // =========================================================================
    // Section 1: Sync Progress
    // =========================================================================
    const syncSection = new PopupMenu.PopupMenuSection();

    // Status label (Idle / Syncing: filename (45%) / Paused)
    const statusItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
    const statusLabel = new St.Label({
        text: _getSyncStatusText(proxies.sync, _),
        y_align: Clutter.ActorAlign.CENTER,
    });
    statusItem.add_child(statusLabel);
    syncSection.addMenuItem(statusItem);

    // Pending changes count
    const pendingItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
    const pendingLabel = new St.Label({
        text: _getPendingText(proxies.sync, _),
        style_class: 'lnxdrive-status-label',
        y_align: Clutter.ActorAlign.CENTER,
    });
    pendingItem.add_child(pendingLabel);
    syncSection.addMenuItem(pendingItem);

    // Last sync time
    const lastSyncItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
    const lastSyncLabel = new St.Label({
        text: _getLastSyncText(proxies.sync, _),
        style_class: 'lnxdrive-status-label',
        y_align: Clutter.ActorAlign.CENTER,
    });
    lastSyncItem.add_child(lastSyncLabel);
    syncSection.addMenuItem(lastSyncItem);

    // Connect SyncProgress signal to update status
    const syncProgressId = proxies.sync.connectSignal(
        'SyncProgress',
        (_proxy, _sender, [file, current, total]) => {
            const percent = total > 0 ? Math.round((current / total) * 100) : 0;
            const basename = file.split('/').pop();
            /* Translators: %s is a filename, %d is a percentage */
            statusLabel.set_text(`${_('Syncing')}: ${basename} (${percent}%)`);
        },
    );
    signalIds.push({proxy: proxies.sync, id: syncProgressId});

    // Connect SyncStarted signal
    const syncStartedId = proxies.sync.connectSignal(
        'SyncStarted',
        () => {
            statusLabel.set_text(`${_('Syncing')}\u2026`);
        },
    );
    signalIds.push({proxy: proxies.sync, id: syncStartedId});

    // Connect SyncCompleted signal
    const syncCompletedId = proxies.sync.connectSignal(
        'SyncCompleted',
        (_proxy, _sender, [filesSynced, errors]) => {
            if (errors > 0)
                /* Translators: %d are counts of synced files and errors */
                statusLabel.set_text(`${_('Completed')}: ${filesSynced} ${_('synced')}, ${errors} ${_('errors')}`);
            else
                statusLabel.set_text(_('Idle'));

            // Refresh pending count and last sync time
            pendingLabel.set_text(_getPendingText(proxies.sync, _));
            lastSyncLabel.set_text(_getLastSyncText(proxies.sync, _));
        },
    );
    signalIds.push({proxy: proxies.sync, id: syncCompletedId});

    // Update labels on property changes
    const syncPropsId = proxies.sync.connect(
        'g-properties-changed',
        (_proxy, changed, _invalidated) => {
            if (changed.lookup_value('PendingChanges', null))
                pendingLabel.set_text(_getPendingText(proxies.sync, _));

            if (changed.lookup_value('LastSyncTime', null))
                lastSyncLabel.set_text(_getLastSyncText(proxies.sync, _));

            const statusVariant = changed.lookup_value('SyncStatus', null);
            if (statusVariant)
                statusLabel.set_text(_getSyncStatusText(proxies.sync, _));
        },
    );
    signalIds.push({proxy: proxies.sync, id: syncPropsId});

    menu.addMenuItem(syncSection);

    // =========================================================================
    // Separator
    // =========================================================================
    menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

    // =========================================================================
    // Section 2: Conflicts
    // =========================================================================
    const conflictsSection = new PopupMenu.PopupMenuSection();

    // Header label for conflicts count
    const conflictsHeaderItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
    const conflictsLabel = new St.Label({
        text: _('No conflicts'),
        y_align: Clutter.ActorAlign.CENTER,
    });
    conflictsHeaderItem.add_child(conflictsLabel);
    conflictsSection.addMenuItem(conflictsHeaderItem);

    // Track conflicts as an array of {path, type} (up to 5 displayed)
    const MAX_VISIBLE_CONFLICTS = 5;
    let conflictEntries = [];
    let conflictMenuItems = [];

    /**
     * Rebuild the per-conflict menu entries from conflictEntries.
     */
    function _rebuildConflictItems() {
        // Remove old items
        for (const item of conflictMenuItems)
            item.destroy();
        conflictMenuItems = [];

        if (conflictEntries.length === 0) {
            conflictsLabel.set_text(_('No conflicts'));
            return;
        }

        const count = conflictEntries.length;
        conflictsLabel.set_text(
            `${count} ${count !== 1 ? _('conflicts detected') : _('conflict detected')}`,
        );

        const visible = conflictEntries.slice(0, MAX_VISIBLE_CONFLICTS);
        for (const entry of visible) {
            const basename = entry.path.split('/').pop();
            const item = new PopupMenu.PopupMenuItem(basename);
            item.connect('activate', () => {
                try {
                    const appInfo = Gio.AppInfo.create_from_commandline(
                        'lnxdrive-preferences --page conflicts',
                        'LNXDrive Preferences',
                        Gio.AppInfoCreateFlags.NONE,
                    );
                    appInfo.launch([], null);
                } catch (e) {
                    console.error(`[LNXDrive] Failed to launch preferences: ${e.message}`);
                }
            });
            conflictsSection.addMenuItem(item);
            conflictMenuItems.push(item);
        }

        if (count > MAX_VISIBLE_CONFLICTS) {
            const moreItem = new PopupMenu.PopupMenuItem(
                `${_('View all')} (${count - MAX_VISIBLE_CONFLICTS} ${_('more')}\u2026)`,
            );
            moreItem.connect('activate', () => {
                try {
                    const appInfo = Gio.AppInfo.create_from_commandline(
                        'lnxdrive-preferences --page conflicts',
                        'LNXDrive Preferences',
                        Gio.AppInfoCreateFlags.NONE,
                    );
                    appInfo.launch([], null);
                } catch (e) {
                    console.error(`[LNXDrive] Failed to launch preferences: ${e.message}`);
                }
            });
            conflictsSection.addMenuItem(moreItem);
            conflictMenuItems.push(moreItem);
        }
    }

    // Listen for new conflicts from the Conflicts interface
    if (proxies.conflicts) {
        const conflictDetectedId = proxies.conflicts.connectSignal(
            'ConflictDetected',
            (_proxy, _sender, [conflictJson]) => {
                try {
                    const data = JSON.parse(conflictJson);
                    const path = data.item_path || data.item_id || 'unknown';
                    conflictEntries.push({id: data.id, path, type: 'content-changed'});
                    _rebuildConflictItems();
                    console.log(`[LNXDrive] Conflict detected: ${path}`);
                } catch (_e) {
                    // Fallback: just increment
                    conflictEntries.push({id: null, path: 'unknown', type: 'unknown'});
                    _rebuildConflictItems();
                }
            },
        );
        signalIds.push({proxy: proxies.conflicts, id: conflictDetectedId});

        // Listen for resolved conflicts
        const conflictResolvedId = proxies.conflicts.connectSignal(
            'ConflictResolved',
            (_proxy, _sender, [conflictId, _strategy]) => {
                // Remove from entries by ID if we have it, otherwise pop last
                conflictEntries = conflictEntries.filter(e => e.id !== conflictId);
                _rebuildConflictItems();
                console.log(`[LNXDrive] Conflict resolved: ${conflictId}`);
            },
        );
        signalIds.push({proxy: proxies.conflicts, id: conflictResolvedId});
    }

    // Fetch initial conflicts from the daemon
    if (proxies.conflicts) {
        proxies.conflicts.ListRemote((result, error) => {
            if (error) {
                console.error(`[LNXDrive] Conflicts.List failed: ${error.message}`);
                return;
            }
            try {
                const [jsonStr] = result;
                const conflicts = JSON.parse(jsonStr);
                if (Array.isArray(conflicts) && conflicts.length > 0) {
                    for (const c of conflicts) {
                        const path = c.item_path || c.path || 'unknown';
                        conflictEntries.push({
                            id: c.id,
                            path,
                            type: 'content-changed',
                        });
                    }
                    _rebuildConflictItems();
                }
            } catch (e) {
                console.error(`[LNXDrive] Failed to parse conflicts: ${e.message}`);
            }
        });
    }

    // Also listen to the legacy ConflictDetected on Sync interface
    const syncConflictDetectedId = proxies.sync.connectSignal(
        'ConflictDetected',
        (_proxy, _sender, [path, conflictType]) => {
            conflictEntries.push({path, type: conflictType});
            _rebuildConflictItems();
            console.log(`[LNXDrive] Conflict detected (sync signal): ${path}`);
        },
    );
    signalIds.push({proxy: proxies.sync, id: syncConflictDetectedId});

    // Reset conflict entries when sync completes without errors
    const conflictResetId = proxies.sync.connectSignal(
        'SyncCompleted',
        (_proxy, _sender, [_filesSynced, errors]) => {
            if (errors === 0) {
                conflictEntries = [];
                _rebuildConflictItems();
            }
        },
    );
    signalIds.push({proxy: proxies.sync, id: conflictResetId});

    menu.addMenuItem(conflictsSection);

    // =========================================================================
    // Separator
    // =========================================================================
    menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

    // =========================================================================
    // Section 3: Connection & Quota
    // =========================================================================
    const quotaSection = new PopupMenu.PopupMenuSection();

    // Connection status
    const connectionItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
    const connectionLabel = new St.Label({
        text: _getConnectionText(proxies.status, _),
        style_class: 'lnxdrive-status-label',
        y_align: Clutter.ActorAlign.CENTER,
    });
    connectionItem.add_child(connectionLabel);
    quotaSection.addMenuItem(connectionItem);

    // Subscribe to ConnectionChanged signal
    const connectionChangedId = proxies.status.connectSignal(
        'ConnectionChanged',
        (_proxy, _sender, [status]) => {
            connectionLabel.set_text(_getConnectionText(proxies.status, _));
        },
    );
    signalIds.push({proxy: proxies.status, id: connectionChangedId});

    // Update connection label on property changes
    const statusPropsId = proxies.status.connect(
        'g-properties-changed',
        (_proxy, changed, _invalidated) => {
            if (changed.lookup_value('ConnectionStatus', null))
                connectionLabel.set_text(_getConnectionText(proxies.status, _));
        },
    );
    signalIds.push({proxy: proxies.status, id: statusPropsId});

    const quotaItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
    const quotaBox = new St.BoxLayout({
        vertical: true,
        x_expand: true,
    });

    const quotaTextLabel = new St.Label({
        text: `${_('Quota')}: ${_('loading\u2026')}`,
        style_class: 'lnxdrive-status-label',
    });
    quotaBox.add_child(quotaTextLabel);

    // Visual progress bar for quota
    const quotaBarOuter = new St.Widget({
        style_class: 'lnxdrive-quota-bar',
        x_expand: true,
        height: 6,
    });

    const quotaBarFill = new St.Widget({
        style_class: 'lnxdrive-quota-fill',
        height: 6,
        width: 0,
    });
    quotaBarOuter.add_child(quotaBarFill);

    quotaBox.add_child(quotaBarOuter);
    quotaItem.add_child(quotaBox);
    quotaSection.addMenuItem(quotaItem);

    // Track the current quota fraction for allocation updates
    let currentQuotaFraction = 0;

    // Update quota bar width when the parent is allocated (responsive to menu width)
    const quotaAllocId = quotaBarOuter.connect('notify::allocation', () => {
        const parentWidth = quotaBarOuter.get_width();
        if (parentWidth > 0 && currentQuotaFraction > 0)
            quotaBarFill.set_width(Math.round(parentWidth * currentQuotaFraction));
    });

    /**
     * Update the quota display with used and total bytes.
     *
     * @param {number} used - Bytes used.
     * @param {number} total - Bytes total.
     */
    function _updateQuota(used, total) {
        quotaTextLabel.set_text(`${_formatBytes(used)} / ${_formatBytes(total)}`);

        if (total > 0)
            currentQuotaFraction = Math.min(used / total, 1.0);
        else
            currentQuotaFraction = 0;

        // Set immediately if parent already has a width
        const parentWidth = quotaBarOuter.get_width();
        if (parentWidth > 0)
            quotaBarFill.set_width(Math.round(parentWidth * currentQuotaFraction));
    }

    // Fetch initial quota
    proxies.status.GetQuotaRemote((result, error) => {
        if (error) {
            console.error(`[LNXDrive] GetQuota failed: ${error.message}`);
            quotaTextLabel.set_text(`${_('Quota')}: ${_('unavailable')}`);
            return;
        }
        const [used, total] = result;
        _updateQuota(used, total);
    });

    // Subscribe to QuotaChanged signal
    const quotaChangedId = proxies.status.connectSignal(
        'QuotaChanged',
        (_proxy, _sender, [used, total]) => {
            _updateQuota(used, total);
        },
    );
    signalIds.push({proxy: proxies.status, id: quotaChangedId});

    menu.addMenuItem(quotaSection);

    // =========================================================================
    // Separator
    // =========================================================================
    menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());

    // =========================================================================
    // Section 4: Actions
    // =========================================================================
    const actionsSection = new PopupMenu.PopupMenuSection();

    // Pause / Resume toggle
    const pauseResumeItem = new PopupMenu.PopupMenuItem(_('Pause Sync'));
    let isPaused = false;

    // Initialize based on current SyncStatus if available
    try {
        const currentStatus = proxies.sync.SyncStatus;
        if (currentStatus === 'paused') {
            isPaused = true;
            pauseResumeItem.label.set_text(_('Resume Sync'));
        }
    } catch (_e) {
        // Property may not be available
    }

    pauseResumeItem.connect('activate', () => {
        if (isPaused) {
            proxies.sync.ResumeRemote((_, error) => {
                if (error)
                    console.error(`[LNXDrive] Resume failed: ${error.message}`);
            });
        } else {
            proxies.sync.PauseRemote((_, error) => {
                if (error)
                    console.error(`[LNXDrive] Pause failed: ${error.message}`);
            });
        }
    });

    // Track paused state from SyncStatus changes
    const pauseStateId = proxies.sync.connect(
        'g-properties-changed',
        (_proxy, changed, _invalidated) => {
            const statusVariant = changed.lookup_value('SyncStatus', null);
            if (statusVariant) {
                const status = statusVariant.unpack();
                if (status === 'paused') {
                    isPaused = true;
                    pauseResumeItem.label.set_text(_('Resume Sync'));
                } else if (isPaused) {
                    isPaused = false;
                    pauseResumeItem.label.set_text(_('Pause Sync'));
                }
            }
        },
    );
    signalIds.push({proxy: proxies.sync, id: pauseStateId});

    actionsSection.addMenuItem(pauseResumeItem);

    // Sync Now
    const syncNowItem = new PopupMenu.PopupMenuItem(_('Sync Now'));
    syncNowItem.connect('activate', () => {
        proxies.sync.SyncNowRemote((_, error) => {
            if (error)
                console.error(`[LNXDrive] SyncNow failed: ${error.message}`);
        });
    });
    actionsSection.addMenuItem(syncNowItem);

    // Preferences - Launch the main preferences application
    const prefsItem = new PopupMenu.PopupMenuItem(_('Preferences'));
    prefsItem.connect('activate', () => {
        try {
            const appInfo = Gio.AppInfo.create_from_commandline(
                'lnxdrive-preferences',
                'LNXDrive Preferences',
                Gio.AppInfoCreateFlags.NONE,
            );
            appInfo.launch([], null);
        } catch (e) {
            console.error(`[LNXDrive] Failed to launch preferences: ${e.message}`);
        }
    });
    actionsSection.addMenuItem(prefsItem);

    menu.addMenuItem(actionsSection);

    return signalIds;
}

/**
 * Get the human-readable sync status text from the proxy.
 *
 * @param {Gio.DBusProxy} syncProxy - The Sync interface proxy.
 * @param {function(string): string} _ - Gettext function.
 * @returns {string} Status text for display.
 */
function _getSyncStatusText(syncProxy, _) {
    try {
        const status = syncProxy.SyncStatus;
        switch (status) {
        case 'idle':
            return _('Idle');
        case 'syncing':
            return `${_('Syncing')}\u2026`;
        case 'paused':
            return _('Paused');
        case 'error':
            return _('Error');
        case 'offline':
            return _('Offline');
        default:
            return status || _('Unknown');
        }
    } catch (_e) {
        return _('Unknown');
    }
}

/**
 * Get the pending changes text from the proxy.
 *
 * @param {Gio.DBusProxy} syncProxy - The Sync interface proxy.
 * @param {function(string): string} _ - Gettext function.
 * @returns {string} Pending changes text for display.
 */
function _getPendingText(syncProxy, _) {
    try {
        const pending = syncProxy.PendingChanges;
        if (pending === 0)
            return _('No pending changes');
        /* Translators: %d is the number of pending changes */
        return `${pending} ${pending !== 1 ? _('pending changes') : _('pending change')}`;
    } catch (_e) {
        return `${_('Pending changes')}: ${_('unknown')}`;
    }
}

/**
 * Get the last sync time text from the proxy.
 *
 * @param {Gio.DBusProxy} syncProxy - The Sync interface proxy.
 * @param {function(string): string} _ - Gettext function.
 * @returns {string} Last sync time text for display.
 */
function _getLastSyncText(syncProxy, _) {
    try {
        const timestamp = syncProxy.LastSyncTime;
        return `${_('Last sync')}: ${_formatLastSyncTime(timestamp, _)}`;
    } catch (_e) {
        return `${_('Last sync')}: ${_('unknown')}`;
    }
}

/**
 * Get the connection status text from the proxy.
 *
 * @param {Gio.DBusProxy} statusProxy - The Status interface proxy.
 * @param {function(string): string} _ - Gettext function.
 * @returns {string} Connection status text for display.
 */
function _getConnectionText(statusProxy, _) {
    try {
        const status = statusProxy.ConnectionStatus;
        switch (status) {
        case 'online':
            return `\u25cf ${_('Online')}`;
        case 'offline':
            return `\u25cb ${_('Offline')}`;
        case 'reconnecting':
            return `\u25d4 ${_('Reconnecting\u2026')}`;
        default:
            return status || _('Unknown');
        }
    } catch (_e) {
        return _('Unknown');
    }
}
