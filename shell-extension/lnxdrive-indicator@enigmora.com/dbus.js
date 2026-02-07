// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Enigmora <https://enigmora.com>

/**
 * LNXDrive D-Bus Proxy Definitions
 *
 * Defines XML introspection strings for the LNXDrive daemon D-Bus interfaces
 * and provides proxy wrappers for communication from the GNOME Shell extension.
 *
 * Bus name:    org.enigmora.LNXDrive
 * Object path: /org/enigmora/LNXDrive
 *
 * Implements: FR-024, FR-025, FR-026
 */

import Gio from 'gi://Gio';

const BUS_NAME = 'org.enigmora.LNXDrive';
const OBJECT_PATH = '/org/enigmora/LNXDrive';

// ---------------------------------------------------------------------------
// org.enigmora.LNXDrive.Sync
// ---------------------------------------------------------------------------
const SyncInterfaceXml = `
<node>
  <interface name="org.enigmora.LNXDrive.Sync">
    <method name="SyncNow"/>
    <method name="Pause"/>
    <method name="Resume"/>

    <property name="SyncStatus" type="s" access="read"/>
    <property name="LastSyncTime" type="x" access="read"/>
    <property name="PendingChanges" type="u" access="read"/>

    <signal name="SyncStarted"/>
    <signal name="SyncCompleted">
      <arg type="u" name="files_synced"/>
      <arg type="u" name="errors"/>
    </signal>
    <signal name="SyncProgress">
      <arg type="s" name="file"/>
      <arg type="u" name="current"/>
      <arg type="u" name="total"/>
    </signal>
    <signal name="ConflictDetected">
      <arg type="s" name="path"/>
      <arg type="s" name="conflict_type"/>
    </signal>
  </interface>
</node>`;

// ---------------------------------------------------------------------------
// org.enigmora.LNXDrive.Status
// ---------------------------------------------------------------------------
const StatusInterfaceXml = `
<node>
  <interface name="org.enigmora.LNXDrive.Status">
    <method name="GetQuota">
      <arg type="t" direction="out" name="used"/>
      <arg type="t" direction="out" name="total"/>
    </method>
    <method name="GetAccountInfo">
      <arg type="a{sv}" direction="out" name="info"/>
    </method>

    <property name="ConnectionStatus" type="s" access="read"/>

    <signal name="QuotaChanged">
      <arg type="t" name="used"/>
      <arg type="t" name="total"/>
    </signal>
    <signal name="ConnectionChanged">
      <arg type="s" name="status"/>
    </signal>
  </interface>
</node>`;

// ---------------------------------------------------------------------------
// org.enigmora.LNXDrive.Manager
// ---------------------------------------------------------------------------
const ManagerInterfaceXml = `
<node>
  <interface name="org.enigmora.LNXDrive.Manager">
    <method name="GetStatus">
      <arg type="s" direction="out" name="status"/>
    </method>

    <property name="Version" type="s" access="read"/>
    <property name="IsRunning" type="b" access="read"/>
  </interface>
</node>`;

/**
 * Create all three D-Bus proxies for communication with the LNXDrive daemon.
 *
 * Proxy wrappers are created inside this function (not at module scope) to
 * comply with GNOME Shell extension lifecycle rules: all D-Bus setup must
 * happen inside enable(), which transitively calls this function.
 *
 * @returns {Promise<{sync: Gio.DBusProxy, status: Gio.DBusProxy, manager: Gio.DBusProxy}|null>}
 *   An object containing all three proxy instances, or null if any proxy
 *   failed to connect (e.g., daemon is not running).
 */
export async function createProxies() {
    try {
        const SyncProxy = Gio.DBusProxy.makeProxyWrapper(SyncInterfaceXml);
        const StatusProxy = Gio.DBusProxy.makeProxyWrapper(StatusInterfaceXml);
        const ManagerProxy = Gio.DBusProxy.makeProxyWrapper(ManagerInterfaceXml);

        const sync = await new Promise((resolve, reject) => {
            SyncProxy(
                Gio.DBus.session,
                BUS_NAME,
                OBJECT_PATH,
                (proxy, error) => {
                    if (error)
                        reject(error);
                    else
                        resolve(proxy);
                },
                null, /* cancellable */
                Gio.DBusProxyFlags.NONE,
            );
        });

        // If the daemon is not running, the proxy has no name owner
        if (sync.g_name_owner === null)
            return null;

        const status = await new Promise((resolve, reject) => {
            StatusProxy(
                Gio.DBus.session,
                BUS_NAME,
                OBJECT_PATH,
                (proxy, error) => {
                    if (error)
                        reject(error);
                    else
                        resolve(proxy);
                },
                null,
                Gio.DBusProxyFlags.NONE,
            );
        });

        const manager = await new Promise((resolve, reject) => {
            ManagerProxy(
                Gio.DBus.session,
                BUS_NAME,
                OBJECT_PATH,
                (proxy, error) => {
                    if (error)
                        reject(error);
                    else
                        resolve(proxy);
                },
                null,
                Gio.DBusProxyFlags.NONE,
            );
        });

        return {sync, status, manager};
    } catch (e) {
        console.error(`[LNXDrive] Failed to create D-Bus proxies: ${e.message}`);
        return null;
    }
}
