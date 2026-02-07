#!/usr/bin/env gjs
// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Enigmora <https://enigmora.com>

/**
 * Integration tests for the LNXDrive GNOME Shell extension D-Bus module.
 *
 * Tests the dbus.js module's createProxies() function against:
 *   1. A running mock daemon (proxies should be valid)
 *   2. A missing daemon (should return null gracefully)
 *   3. Basic proxy signal subscription capability
 *
 * Usage:
 *     # With mock daemon running:
 *     python3 tests/mock-dbus-daemon.py --authenticated &
 *     gjs tests/test-shell-extension.js
 *
 *     # Without daemon (tests graceful handling):
 *     gjs tests/test-shell-extension.js --no-daemon
 *
 * NOTE: This script runs outside of GNOME Shell, so Shell-specific imports
 * (St, Clutter, PanelMenu, etc.) are NOT available. We test only the dbus.js
 * module which depends solely on Gio.
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

/**
 * Run a named test function and report the result.
 *
 * @param {string} name - Test name.
 * @param {Function} fn - Test function (may be async).
 */
async function runTest(name, fn) {
    totalTests++;
    try {
        await fn();
        passedTests++;
        print(`  PASS: ${name}`);
    } catch (e) {
        failedTests++;
        print(`  FAIL: ${name}`);
        print(`        ${e.message || e}`);
    }
}

/**
 * Simple assertion function.
 *
 * @param {boolean} condition - Condition to assert.
 * @param {string} message - Error message on failure.
 */
function assert(condition, message) {
    if (!condition)
        throw new Error(`Assertion failed: ${message}`);
}

/**
 * Assert that two values are strictly equal.
 *
 * @param {*} actual - The actual value.
 * @param {*} expected - The expected value.
 * @param {string} [message] - Optional error message.
 */
function assertEqual(actual, expected, message) {
    if (actual !== expected) {
        const msg = message || `Expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`;
        throw new Error(msg);
    }
}

// ---------------------------------------------------------------------------
// D-Bus constants (must match dbus.js and the mock daemon)
// ---------------------------------------------------------------------------
const BUS_NAME = 'org.enigmora.LNXDrive';
const OBJECT_PATH = '/org/enigmora/LNXDrive';

// ---------------------------------------------------------------------------
// Import the dbus.js module
// ---------------------------------------------------------------------------

// The dbus.js module is in the shell extension directory.  We import it
// relative to this test file's location.
const SCRIPT_DIR = GLib.path_get_dirname(
    GLib.filename_from_uri(import.meta.url)[0]
);
const EXTENSION_DIR = GLib.build_filenamev([
    SCRIPT_DIR, '..', 'shell-extension', 'lnxdrive-indicator@enigmora.com',
]);

// Dynamic import of the dbus module
let dbusModule = null;
try {
    // ESM dynamic import using file:// URI
    const dbusPath = GLib.build_filenamev([EXTENSION_DIR, 'dbus.js']);
    const dbusUri = GLib.filename_to_uri(dbusPath, null);
    dbusModule = await import(dbusUri);
} catch (e) {
    print(`ERROR: Could not import dbus.js from ${EXTENSION_DIR}`);
    print(`       ${e.message}`);
    print('       Make sure you are running from the project root directory.');
    // Exit with error code
    imports.system.exit(1);
}

// ---------------------------------------------------------------------------
// Check whether the daemon is running
// ---------------------------------------------------------------------------
function isDaemonRunning() {
    try {
        const bus = Gio.bus_get_sync(Gio.BusType.SESSION, null);
        const result = bus.call_sync(
            'org.freedesktop.DBus',
            '/org/freedesktop/DBus',
            'org.freedesktop.DBus',
            'NameHasOwner',
            new GLib.Variant('(s)', [BUS_NAME]),
            new GLib.VariantType('(b)'),
            Gio.DBusCallFlags.NONE,
            5000,
            null,
        );
        return result.get_child_value(0).get_boolean();
    } catch (_e) {
        return false;
    }
}

// Parse command-line arguments
const args = ARGV;
const noDaemonMode = args.includes('--no-daemon');

// ---------------------------------------------------------------------------
// Test execution
// ---------------------------------------------------------------------------

print('');
print('LNXDrive Shell Extension D-Bus Tests');
print('=====================================');
print('');

const daemonAvailable = isDaemonRunning();

if (noDaemonMode) {
    print('Mode: --no-daemon (testing graceful handling when daemon is absent)');
    print('');

    // ----- Test: createProxies returns null when daemon is absent -----
    await runTest('createProxies returns null when daemon is absent', async () => {
        // If the daemon happens to be running in no-daemon mode, skip
        if (daemonAvailable) {
            print('        SKIP: daemon is actually running; cannot test absence');
            return;
        }

        const proxies = await dbusModule.createProxies();

        // createProxies should return null (not throw) when daemon is absent
        assertEqual(proxies, null,
            'createProxies should return null when the daemon is not on the bus');
    });

    // ----- Test: createProxies does not throw when daemon is absent -----
    await runTest('createProxies does not throw when daemon is absent', async () => {
        if (daemonAvailable) {
            print('        SKIP: daemon is actually running; cannot test absence');
            return;
        }

        let threw = false;
        try {
            await dbusModule.createProxies();
        } catch (_e) {
            threw = true;
        }

        assert(!threw,
            'createProxies should handle missing daemon gracefully without throwing');
    });
} else {
    if (!daemonAvailable) {
        print('WARNING: Mock daemon is not running on the session bus.');
        print('         Start it with: python3 tests/mock-dbus-daemon.py --authenticated &');
        print('         Or run with --no-daemon to test absence handling.');
        print('');
        // Run absence tests anyway
        await runTest('createProxies returns null when daemon is absent', async () => {
            const proxies = await dbusModule.createProxies();
            assertEqual(proxies, null,
                'createProxies should return null when the daemon is not on the bus');
        });
    } else {
        print('Mode: daemon running (testing proxy creation and signal capability)');
        print('');

        // ----- Test: createProxies returns valid proxy objects -----
        await runTest('createProxies returns valid proxy objects', async () => {
            const proxies = await dbusModule.createProxies();

            assert(proxies !== null, 'createProxies should not return null');
            assert(typeof proxies === 'object', 'createProxies should return an object');

            assert(proxies.sync !== null && proxies.sync !== undefined,
                'proxies.sync should exist');
            assert(proxies.status !== null && proxies.status !== undefined,
                'proxies.status should exist');
            assert(proxies.manager !== null && proxies.manager !== undefined,
                'proxies.manager should exist');
        });

        // ----- Test: proxies have correct interface names -----
        await runTest('proxies have correct D-Bus interface names', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            assertEqual(proxies.sync.g_interface_name, 'org.enigmora.LNXDrive.Sync',
                'sync proxy interface name mismatch');
            assertEqual(proxies.status.g_interface_name, 'org.enigmora.LNXDrive.Status',
                'status proxy interface name mismatch');
            assertEqual(proxies.manager.g_interface_name, 'org.enigmora.LNXDrive.Manager',
                'manager proxy interface name mismatch');
        });

        // ----- Test: proxies connect to correct bus name -----
        await runTest('proxies connect to correct bus name', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            assertEqual(proxies.sync.g_name, BUS_NAME,
                'sync proxy bus name mismatch');
            assertEqual(proxies.status.g_name, BUS_NAME,
                'status proxy bus name mismatch');
            assertEqual(proxies.manager.g_name, BUS_NAME,
                'manager proxy bus name mismatch');
        });

        // ----- Test: proxies connect to correct object path -----
        await runTest('proxies connect to correct object path', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            assertEqual(proxies.sync.g_object_path, OBJECT_PATH,
                'sync proxy object path mismatch');
            assertEqual(proxies.status.g_object_path, OBJECT_PATH,
                'status proxy object path mismatch');
            assertEqual(proxies.manager.g_object_path, OBJECT_PATH,
                'manager proxy object path mismatch');
        });

        // ----- Test: sync proxy has SyncStatus property -----
        await runTest('sync proxy can read SyncStatus property', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const status = proxies.sync.SyncStatus;
            assert(typeof status === 'string',
                `SyncStatus should be a string, got ${typeof status}`);

            const validStatuses = ['idle', 'syncing', 'paused', 'error', 'offline'];
            assert(validStatuses.includes(status),
                `SyncStatus '${status}' not in valid set: ${validStatuses.join(', ')}`);
        });

        // ----- Test: status proxy can call GetQuota -----
        await runTest('status proxy can call GetQuota', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.status.GetQuotaRemote((result, error) => {
                    if (error) {
                        reject(new Error(`GetQuota failed: ${error.message}`));
                        return;
                    }

                    const [used, total] = result;
                    assert(typeof used === 'number' || typeof used === 'bigint',
                        `used should be a number, got ${typeof used}`);
                    assert(typeof total === 'number' || typeof total === 'bigint',
                        `total should be a number, got ${typeof total}`);
                    assert(total > 0, 'total quota should be > 0');
                    resolve();
                });
            });
        });

        // ----- Test: proxy signal subscription capability -----
        await runTest('sync proxy supports signal subscription (connectSignal)', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            // Verify that connectSignal is a function on the proxy
            assert(typeof proxies.sync.connectSignal === 'function',
                'sync proxy should have a connectSignal method');

            // Subscribe and immediately unsubscribe to verify it does not throw
            const handlerId = proxies.sync.connectSignal('SyncStarted', () => {});
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            // Clean up
            proxies.sync.disconnectSignal(handlerId);
        });

        // ----- Test: proxy property change notification capability -----
        await runTest('sync proxy supports property change signals', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            // Verify that connect('g-properties-changed', ...) works
            assert(typeof proxies.sync.connect === 'function',
                'sync proxy should have a connect method');

            const handlerId = proxies.sync.connect('g-properties-changed', () => {});
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connect should return a positive handler ID, got ${handlerId}`);

            // Clean up
            proxies.sync.disconnect(handlerId);
        });

        // ----- Test: manager proxy can read Version property -----
        await runTest('manager proxy can read Version property', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const version = proxies.manager.Version;
            assert(typeof version === 'string',
                `Version should be a string, got ${typeof version}`);
            assert(version.length > 0, 'Version should not be empty');
        });

        // ----- Test: conflicts proxy exists and has correct interface -----
        await runTest('conflicts proxy exists and has correct interface name', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            assert(proxies.conflicts !== null && proxies.conflicts !== undefined,
                'proxies.conflicts should exist');
            assertEqual(proxies.conflicts.g_interface_name,
                'org.enigmora.LNXDrive.Conflicts',
                'conflicts proxy interface name mismatch');
        });

        // ----- Test: conflicts proxy can call List -----
        await runTest('conflicts proxy can call List', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.conflicts.ListRemote((result, error) => {
                    if (error) {
                        reject(new Error(`Conflicts.List failed: ${error.message}`));
                        return;
                    }

                    const [jsonStr] = result;
                    assert(typeof jsonStr === 'string',
                        `List should return a string, got ${typeof jsonStr}`);

                    const conflicts = JSON.parse(jsonStr);
                    assert(Array.isArray(conflicts),
                        'List result should be a JSON array');
                    assert(conflicts.length > 0,
                        'Mock daemon should have at least one conflict');

                    // Check first conflict has expected fields
                    const first = conflicts[0];
                    assert(first.id !== undefined, 'conflict should have id');
                    assert(first.item_id !== undefined, 'conflict should have item_id');
                    assert(first.local_version !== undefined,
                        'conflict should have local_version');
                    assert(first.remote_version !== undefined,
                        'conflict should have remote_version');
                    resolve();
                });
            });
        });

        // ----- Test: conflicts proxy can call Resolve -----
        await runTest('conflicts proxy can call Resolve', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.conflicts.ResolveRemote(
                    'conflict-001', 'keep_local',
                    (result, error) => {
                        if (error) {
                            reject(new Error(`Conflicts.Resolve failed: ${error.message}`));
                            return;
                        }

                        const [success] = result;
                        assert(typeof success === 'boolean',
                            `Resolve should return a boolean, got ${typeof success}`);
                        resolve();
                    },
                );
            });
        });

        // ----- Test: conflicts proxy supports ConflictDetected signal -----
        await runTest('conflicts proxy supports ConflictDetected signal subscription', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const handlerId = proxies.conflicts.connectSignal(
                'ConflictDetected', () => {},
            );
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            proxies.conflicts.disconnectSignal(handlerId);
        });
    }
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
print('');
print('-------------------------------------');
print(`Results: ${passedTests} passed, ${failedTests} failed, ${totalTests} total`);
print('');

if (failedTests > 0)
    imports.system.exit(1);
