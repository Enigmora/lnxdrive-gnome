// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Enigmora <https://enigmora.com>

/**
 * LNXDrive GNOME Shell Extension - Entry Point
 *
 * Provides a persistent status indicator in the GNOME Shell top bar
 * showing sync progress, conflicts, quota, and quick actions.
 *
 * Implements: FR-009, FR-012, FR-025, FR-028
 * Compatibility: GNOME Shell 45, 46, 47
 *
 * IMPORTANT: Do NOT import Gdk, Gtk, or Adw here.
 * Only Shell-safe imports are allowed in extension.js.
 */

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {LnxdriveIndicator} from './indicator.js';

const INDICATOR_NAME = 'lnxdrive-indicator';

export default class LnxdriveExtension extends Extension {
    /**
     * Called when the extension is enabled.
     * Creates the indicator and adds it to the GNOME Shell status area.
     */
    enable() {
        this._indicator = new LnxdriveIndicator(this);
        Main.panel.addToStatusArea(INDICATOR_NAME, this._indicator);
    }

    /**
     * Called when the extension is disabled or GNOME Shell locks the screen.
     * Destroys the indicator and releases all resources.
     */
    disable() {
        if (this._indicator) {
            this._indicator.destroy();
            this._indicator = null;
        }
    }
}
