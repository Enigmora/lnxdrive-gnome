// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Enigmora <https://enigmora.com>

/**
 * LNXDrive Panel Indicator
 *
 * Persistent icon in the GNOME Shell top bar with a dropdown menu showing
 * sync progress, conflicts, quota information, and quick actions.
 *
 * Implements: FR-009, FR-010, FR-011, FR-012, FR-025, FR-026, FR-028
 * Success criteria: SC-003, SC-007, SC-008
 */

import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';

import {createProxies} from './dbus.js';
import {buildMenu} from './menuItems.js';

/**
 * Module-level gettext function, set by _init() from the Extension instance.
 * @type {function(string): string}
 */
let _ = s => s;

/** CSS style classes that represent icon states. */
const STATE_CLASSES = [
    'lnxdrive-syncing',
    'lnxdrive-paused',
    'lnxdrive-error',
    'lnxdrive-offline',
];

/** Delay in seconds before retrying proxy connection. */
const RETRY_DELAY_SECS = 5;

export const LnxdriveIndicator = GObject.registerClass(
class LnxdriveIndicator extends PanelMenu.Button {
    /**
     * @param {import('resource:///org/gnome/shell/extensions/extension.js').Extension} extension
     */
    _init(extension) {
        super._init(0.0, 'LNXDrive Sync Indicator', false);

        this._extension = extension;
        this._proxies = null;
        this._signalIds = [];
        this._menuSignalIds = [];
        this._retrySourceId = 0;

        // Set up gettext for translatable strings in this module and menuItems
        _ = extension.gettext.bind(extension);

        // Create the status icon (symbolic icon for the top bar)
        this._icon = new St.Icon({
            icon_name: 'com.enigmora.LNXDrive-symbolic',
            style_class: 'system-status-icon',
        });
        this.add_child(this._icon);

        // Start proxy initialization
        this._initProxies().catch(e => {
            console.error(`[LNXDrive] Proxy init error: ${e.message}`);
        });
    }

    /**
     * Initialize D-Bus proxies and set up signal connections.
     * If the daemon is unavailable, sets the indicator to offline state
     * and retries after RETRY_DELAY_SECS.
     */
    async _initProxies() {
        // Clear any pending retry
        this._clearRetryTimeout();

        this._proxies = await createProxies();

        if (!this._proxies) {
            this._setOfflineState();
            this._scheduleRetry();
            return;
        }

        this._buildMenuAndConnect();
    }

    /**
     * Build the dropdown menu and connect all D-Bus signal handlers.
     */
    _buildMenuAndConnect() {
        // Build the menu; collect signal handler IDs from menu construction
        this._menuSignalIds = buildMenu(this.menu, this._proxies, _);

        // --- Sync status property change ---
        const syncStatusId = this._proxies.sync.connect(
            'g-properties-changed',
            (_proxy, changed, _invalidated) => {
                const statusVariant = changed.lookup_value('SyncStatus', null);
                if (statusVariant)
                    this._updateIconState(statusVariant.unpack());
            },
        );
        this._signalIds.push({proxy: this._proxies.sync, id: syncStatusId});

        // Read current SyncStatus to set initial icon state
        try {
            const currentStatus = this._proxies.sync.SyncStatus;
            if (currentStatus)
                this._updateIconState(currentStatus);
        } catch (_e) {
            // Property may not be available yet; leave default state
        }

        // --- Daemon name-owner monitoring (FR-025, SC-008) ---
        // When the daemon disappears from the bus, the proxy's g-name-owner
        // becomes null. When it reappears, it gets a new value.
        const nameOwnerId = this._proxies.sync.connect(
            'notify::g-name-owner',
            proxy => {
                const owner = proxy.g_name_owner;
                if (owner === null || owner === '')
                    this._onDaemonLost();
                else
                    this._onDaemonFound();
            },
        );
        this._signalIds.push({proxy: this._proxies.sync, id: nameOwnerId});
    }

    /**
     * Update the icon visual state based on the daemon's SyncStatus property.
     *
     * States: idle, syncing, paused, error, offline
     *
     * @param {string} status - The current sync status string from D-Bus.
     */
    _updateIconState(status) {
        // Remove all state classes from the indicator button itself
        for (const cls of STATE_CLASSES)
            this.remove_style_class_name(cls);

        switch (status) {
        case 'idle':
            // No additional class; default appearance
            break;
        case 'syncing':
            this.add_style_class_name('lnxdrive-syncing');
            break;
        case 'paused':
            this.add_style_class_name('lnxdrive-paused');
            break;
        case 'error':
            this.add_style_class_name('lnxdrive-error');
            break;
        case 'offline':
            this.add_style_class_name('lnxdrive-offline');
            break;
        default:
            console.warn(`[LNXDrive] Unknown sync status: ${status}`);
            break;
        }
    }

    /**
     * Handle daemon disappearing from the session bus.
     * Sets the indicator to offline state and shows an informational message.
     */
    _onDaemonLost() {
        console.log('[LNXDrive] Daemon lost from session bus');
        this._setOfflineState();
        this._scheduleRetry();
    }

    /**
     * Handle daemon reappearing on the session bus.
     * Re-creates proxies and rebuilds the menu. (SC-008: <10s recovery)
     */
    _onDaemonFound() {
        console.log('[LNXDrive] Daemon found on session bus');
        this._clearRetryTimeout();
        this._disconnectAll();
        this._initProxies().catch(e => {
            console.error(`[LNXDrive] Re-init error: ${e.message}`);
        });
    }

    /**
     * Set the indicator to offline state.
     * Removes all state classes, adds offline class, and rebuilds the menu
     * with a "Daemon not running" informational item.
     */
    _setOfflineState() {
        for (const cls of STATE_CLASSES)
            this.remove_style_class_name(cls);
        this.add_style_class_name('lnxdrive-offline');

        // Clear existing menu
        this.menu.removeAll();

        // Add an informational section showing daemon is offline
        const offlineSection = new PopupMenu.PopupMenuSection();

        const offlineItem = new PopupMenu.PopupBaseMenuItem({reactive: false});
        const box = new St.BoxLayout({
            vertical: true,
            x_expand: true,
        });

        const titleLabel = new St.Label({
            text: _('Daemon not running'),
            style_class: 'lnxdrive-status-label',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(titleLabel);

        const waitingLabel = new St.Label({
            text: _('Waiting for daemon\u2026'),
            style_class: 'lnxdrive-status-label',
            x_align: Clutter.ActorAlign.CENTER,
        });
        box.add_child(waitingLabel);

        offlineItem.add_child(box);
        offlineSection.addMenuItem(offlineItem);
        this.menu.addMenuItem(offlineSection);
    }

    /**
     * Schedule a retry attempt to connect to the daemon.
     * Uses GLib.timeout_add_seconds for proper GNOME Shell integration.
     */
    _scheduleRetry() {
        this._clearRetryTimeout();

        this._retrySourceId = GLib.timeout_add_seconds(
            GLib.PRIORITY_DEFAULT,
            RETRY_DELAY_SECS,
            () => {
                this._retrySourceId = 0;
                this._initProxies().catch(e => {
                    console.error(`[LNXDrive] Retry error: ${e.message}`);
                });
                return GLib.SOURCE_REMOVE;
            },
        );
    }

    /**
     * Clear any pending retry timeout.
     */
    _clearRetryTimeout() {
        if (this._retrySourceId) {
            GLib.Source.remove(this._retrySourceId);
            this._retrySourceId = 0;
        }
    }

    /**
     * Disconnect all tracked D-Bus signal handlers.
     */
    _disconnectAll() {
        for (const {proxy, id} of this._signalIds) {
            try {
                proxy.disconnect(id);
            } catch (_e) {
                // Proxy may already be finalized
            }
        }
        this._signalIds = [];

        for (const {proxy, id} of this._menuSignalIds) {
            try {
                proxy.disconnect(id);
            } catch (_e) {
                // Proxy may already be finalized
            }
        }
        this._menuSignalIds = [];
    }

    /**
     * Clean up all resources.
     * Called by the extension's disable() and by GNOME Shell on lock screen.
     */
    destroy() {
        this._clearRetryTimeout();
        this._disconnectAll();
        this._proxies = null;
        super.destroy();
    }
});
