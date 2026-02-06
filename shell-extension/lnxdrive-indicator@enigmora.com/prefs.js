// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Enigmora <https://enigmora.com>

/**
 * LNXDrive Extension Preferences
 *
 * Minimal preferences window for the GNOME Shell extension.
 * Provides a single "Full Settings" button that launches the main
 * LNXDrive preferences application (implemented in Rust with gtk4-rs).
 *
 * NOTE: Gtk, Adw, and Gdk are allowed ONLY in prefs.js (not in extension.js).
 */

import Adw from 'gi://Adw';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class LnxdrivePreferences extends ExtensionPreferences {
    /**
     * Populate the preferences window with LNXDrive settings.
     *
     * @param {Adw.PreferencesWindow} window - The preferences window to populate.
     */
    fillPreferencesWindow(window) {
        const _ = this.gettext.bind(this);

        const page = new Adw.PreferencesPage({
            title: 'LNXDrive',
            icon_name: 'com.enigmora.LNXDrive-symbolic',
        });

        const group = new Adw.PreferencesGroup({
            title: _('Settings'),
            description: _('Configure LNXDrive sync behavior and account settings.'),
        });

        // Action row with a button to open the full preferences application
        const settingsRow = new Adw.ActionRow({
            title: _('Full Settings'),
            subtitle: _('Open the complete LNXDrive preferences application'),
            activatable: true,
        });

        const openButton = new Gtk.Button({
            label: _('Open'),
            valign: Gtk.Align.CENTER,
            css_classes: ['suggested-action'],
        });

        openButton.connect('clicked', () => {
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

        settingsRow.add_suffix(openButton);
        settingsRow.set_activatable_widget(openButton);

        group.add(settingsRow);
        page.add(group);
        window.add(page);
    }
}
