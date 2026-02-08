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

        // ----- Test: status proxy has ConnectionStatus property -----
        await runTest('status proxy can read ConnectionStatus property', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const connStatus = proxies.status.ConnectionStatus;
            assert(typeof connStatus === 'string',
                `ConnectionStatus should be a string, got ${typeof connStatus}`);

            const validStatuses = ['online', 'offline', 'reconnecting'];
            assert(validStatuses.includes(connStatus),
                `ConnectionStatus '${connStatus}' not in valid set: ${validStatuses.join(', ')}`);
        });

        // ----- Test: status proxy supports ConnectionChanged signal -----
        await runTest('status proxy supports ConnectionChanged signal subscription', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const handlerId = proxies.status.connectSignal(
                'ConnectionChanged', () => {},
            );
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            proxies.status.disconnectSignal(handlerId);
        });

        // ----- Test: sync proxy has LastSyncTime property -----
        await runTest('sync proxy can read LastSyncTime property', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const lastSync = proxies.sync.LastSyncTime;
            assert(typeof lastSync === 'number' || typeof lastSync === 'bigint',
                `LastSyncTime should be a number, got ${typeof lastSync}`);
        });

        // ----- Test: sync proxy has PendingChanges property -----
        await runTest('sync proxy can read PendingChanges property', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const pending = proxies.sync.PendingChanges;
            assert(typeof pending === 'number' || typeof pending === 'bigint',
                `PendingChanges should be a number, got ${typeof pending}`);
        });

        // ----- Test: sync proxy can call SyncNow -----
        await runTest('sync proxy can call SyncNow', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.sync.SyncNowRemote((_, error) => {
                    if (error)
                        reject(new Error(`SyncNow failed: ${error.message}`));
                    else
                        resolve();
                });
            });
        });

        // ----- Test: sync proxy can call Pause -----
        await runTest('sync proxy can call Pause', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.sync.PauseRemote((_, error) => {
                    if (error)
                        reject(new Error(`Pause failed: ${error.message}`));
                    else
                        resolve();
                });
            });
        });

        // ----- Test: sync proxy can call Resume -----
        await runTest('sync proxy can call Resume', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.sync.ResumeRemote((_, error) => {
                    if (error)
                        reject(new Error(`Resume failed: ${error.message}`));
                    else
                        resolve();
                });
            });
        });

        // ----- Test: status proxy can call GetAccountInfo -----
        await runTest('status proxy can call GetAccountInfo', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.status.GetAccountInfoRemote((result, error) => {
                    if (error) {
                        reject(new Error(`GetAccountInfo failed: ${error.message}`));
                        return;
                    }

                    const [info] = result;
                    assert(typeof info === 'object',
                        `GetAccountInfo should return an object, got ${typeof info}`);
                    // The mock returns {email, display_name, provider}
                    assert(info.email !== undefined,
                        'GetAccountInfo result should contain email');
                    resolve();
                });
            });
        });

        // ----- Test: manager proxy can call GetStatus -----
        await runTest('manager proxy can call GetStatus', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.manager.GetStatusRemote((result, error) => {
                    if (error) {
                        reject(new Error(`GetStatus failed: ${error.message}`));
                        return;
                    }

                    const [status] = result;
                    assert(typeof status === 'string',
                        `GetStatus should return a string, got ${typeof status}`);
                    const validStatuses = ['running', 'stopped'];
                    assert(validStatuses.includes(status),
                        `GetStatus '${status}' not in valid set: ${validStatuses.join(', ')}`);
                    resolve();
                });
            });
        });

        // ----- Test: manager proxy can read IsRunning property -----
        await runTest('manager proxy can read IsRunning property', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const isRunning = proxies.manager.IsRunning;
            assert(typeof isRunning === 'boolean',
                `IsRunning should be a boolean, got ${typeof isRunning}`);
        });

        // ----- Test: conflicts proxy can call GetDetails -----
        await runTest('conflicts proxy can call GetDetails', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.conflicts.GetDetailsRemote(
                    'conflict-001',
                    (result, error) => {
                        if (error) {
                            reject(new Error(`GetDetails failed: ${error.message}`));
                            return;
                        }

                        const [jsonStr] = result;
                        assert(typeof jsonStr === 'string',
                            `GetDetails should return a string, got ${typeof jsonStr}`);

                        const details = JSON.parse(jsonStr);
                        assert(details.id === 'conflict-001',
                            `Expected conflict id 'conflict-001', got '${details.id}'`);
                        resolve();
                    },
                );
            });
        });

        // ----- Test: conflicts proxy can call ResolveAll -----
        await runTest('conflicts proxy can call ResolveAll', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            await new Promise((resolve, reject) => {
                proxies.conflicts.ResolveAllRemote(
                    'keep_local',
                    (result, error) => {
                        if (error) {
                            reject(new Error(`ResolveAll failed: ${error.message}`));
                            return;
                        }

                        const [count] = result;
                        assert(typeof count === 'number',
                            `ResolveAll should return a number, got ${typeof count}`);
                        resolve();
                    },
                );
            });
        });

        // ----- Test: status proxy supports QuotaChanged signal -----
        await runTest('status proxy supports QuotaChanged signal subscription', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const handlerId = proxies.status.connectSignal(
                'QuotaChanged', () => {},
            );
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            proxies.status.disconnectSignal(handlerId);
        });

        // ----- Test: sync proxy supports SyncCompleted signal -----
        await runTest('sync proxy supports SyncCompleted signal subscription', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const handlerId = proxies.sync.connectSignal(
                'SyncCompleted', () => {},
            );
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            proxies.sync.disconnectSignal(handlerId);
        });

        // ----- Test: sync proxy supports SyncProgress signal -----
        await runTest('sync proxy supports SyncProgress signal subscription', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const handlerId = proxies.sync.connectSignal(
                'SyncProgress', () => {},
            );
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            proxies.sync.disconnectSignal(handlerId);
        });

        // ----- Test: conflicts proxy supports ConflictResolved signal -----
        await runTest('conflicts proxy supports ConflictResolved signal subscription', async () => {
            const proxies = await dbusModule.createProxies();
            assert(proxies !== null, 'proxies should not be null');

            const handlerId = proxies.conflicts.connectSignal(
                'ConflictResolved', () => {},
            );
            assert(typeof handlerId === 'number' && handlerId > 0,
                `connectSignal should return a positive handler ID, got ${handlerId}`);

            proxies.conflicts.disconnectSignal(handlerId);
        });
    }
}

// ---------------------------------------------------------------------------
// Performance benchmarks (SC-005: <500ms for 5000+ files)
// ---------------------------------------------------------------------------

if (daemonAvailable && !noDaemonMode) {
    print('');
    print('Performance Benchmarks');
    print('-------------------------------------');

    // ----- Benchmark: GetBatchFileStatus with 5000 files -----
    await runTest('PERF: GetBatchFileStatus 5000 files completes in <500ms (SC-005)', async () => {
        // Create a Files proxy via raw Gio (not part of dbus.js createProxies)
        const FilesXml = `
        <node>
          <interface name="org.enigmora.LNXDrive.Files">
            <method name="GetBatchFileStatus">
              <arg type="as" direction="in" name="paths"/>
              <arg type="a{ss}" direction="out" name="statuses"/>
            </method>
          </interface>
        </node>`;

        const FilesProxy = Gio.DBusProxy.makeProxyWrapper(FilesXml);
        const filesProxy = await new Promise((resolve, reject) => {
            FilesProxy(
                Gio.DBus.session,
                BUS_NAME,
                OBJECT_PATH,
                (proxy, error) => {
                    if (error) reject(error);
                    else resolve(proxy);
                },
                null,
                Gio.DBusProxyFlags.NONE,
            );
        });

        // Generate 5000 file paths
        const paths = [];
        for (let i = 0; i < 5000; i++)
            paths.push(`/home/user/OneDrive/file-${i}.txt`);

        const startTime = GLib.get_monotonic_time();

        await new Promise((resolve, reject) => {
            filesProxy.GetBatchFileStatusRemote(paths, (result, error) => {
                if (error) {
                    reject(new Error(`GetBatchFileStatus failed: ${error.message}`));
                    return;
                }
                resolve(result);
            });
        });

        const elapsed = (GLib.get_monotonic_time() - startTime) / 1000; // microseconds to ms
        print(`        Elapsed: ${elapsed.toFixed(1)}ms for 5000 files`);

        assert(elapsed < 500,
            `GetBatchFileStatus took ${elapsed.toFixed(1)}ms, exceeds SC-005 limit of 500ms`);
    });

    // ----- Benchmark: Proxy creation latency -----
    await runTest('PERF: createProxies completes in <200ms', async () => {
        const startTime = GLib.get_monotonic_time();
        const proxies = await dbusModule.createProxies();
        const elapsed = (GLib.get_monotonic_time() - startTime) / 1000;

        assert(proxies !== null, 'proxies should not be null');
        print(`        Elapsed: ${elapsed.toFixed(1)}ms for proxy creation`);

        assert(elapsed < 200,
            `createProxies took ${elapsed.toFixed(1)}ms, should be <200ms`);
    });

    // ----- Benchmark: Conflicts.List with JSON parsing -----
    await runTest('PERF: Conflicts.List + JSON parse completes in <100ms', async () => {
        const proxies = await dbusModule.createProxies();
        assert(proxies !== null, 'proxies should not be null');

        const startTime = GLib.get_monotonic_time();

        await new Promise((resolve, reject) => {
            proxies.conflicts.ListRemote((result, error) => {
                if (error) {
                    reject(new Error(`Conflicts.List failed: ${error.message}`));
                    return;
                }
                const [jsonStr] = result;
                JSON.parse(jsonStr); // parse overhead included
                resolve();
            });
        });

        const elapsed = (GLib.get_monotonic_time() - startTime) / 1000;
        print(`        Elapsed: ${elapsed.toFixed(1)}ms for List + parse`);

        assert(elapsed < 100,
            `Conflicts.List + parse took ${elapsed.toFixed(1)}ms, should be <100ms`);
    });
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
