/* lnxdrive-dbus-client.h — D-Bus client for communication with lnxdrive-daemon
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Terminology glossary (keep in sync with lnxdrive-guide):
 *   CloudOnly   = cloud-only (D-Bus status) = placeholder (user-facing)
 *   Synced      = file fully downloaded and pinned locally
 *   PinFile     = pin + hydrate (download and keep local)
 *   UnpinFile   = unpin + dehydrate (convert to placeholder, free disk space)
 *   SyncPath    = force immediate sync of a file/directory
 */

#ifndef LNXDRIVE_DBUS_CLIENT_H
#define LNXDRIVE_DBUS_CLIENT_H

#include <gio/gio.h>

G_BEGIN_DECLS

#define LNXDRIVE_TYPE_DBUS_CLIENT (lnxdrive_dbus_client_get_type ())

G_DECLARE_FINAL_TYPE (LnxdriveDbusClient, lnxdrive_dbus_client,
                      LNXDRIVE, DBUS_CLIENT, GObject)

/* ---------------------------------------------------------------------------
 * D-Bus constants
 * ---------------------------------------------------------------------------*/
#define LNXDRIVE_DBUS_BUS_NAME    "org.enigmora.LNXDrive"
#define LNXDRIVE_DBUS_OBJECT_PATH "/org/enigmora/LNXDrive"
#define LNXDRIVE_DBUS_IFACE_FILES "org.enigmora.LNXDrive.Files"

/* D-Bus error domains */
#define LNXDRIVE_DBUS_ERROR_INSUFFICIENT_DISK_SPACE \
    "org.enigmora.LNXDrive.Error.InsufficientDiskSpace"
#define LNXDRIVE_DBUS_ERROR_FILE_IN_USE \
    "org.enigmora.LNXDrive.Error.FileInUse"
#define LNXDRIVE_DBUS_ERROR_INVALID_PATH \
    "org.enigmora.LNXDrive.Error.InvalidPath"

/* ---------------------------------------------------------------------------
 * Callback type for requesting Nautilus to re-read file info.
 * ---------------------------------------------------------------------------*/
typedef void (*LnxdriveInvalidateFunc) (gpointer user_data);

/* ---------------------------------------------------------------------------
 * Public API
 * ---------------------------------------------------------------------------*/

/* Singleton accessor. */
LnxdriveDbusClient *lnxdrive_dbus_client_get_default (void);

/* Release the singleton (call from nautilus_module_shutdown). */
void                lnxdrive_dbus_client_release_default (void);

/* Get a single file status from the local cache.
 * Returns a static string such as "synced", "cloud-only", "unknown", etc.
 * The returned string is owned by the cache and must NOT be freed. */
const char         *lnxdrive_dbus_client_get_file_status (LnxdriveDbusClient *self,
                                                          const char         *path);

/* Batch-query file statuses over D-Bus (synchronous).
 * Returns a GHashTable mapping (char* path -> char* status).
 * The caller owns the returned table; call g_hash_table_unref(). */
GHashTable         *lnxdrive_dbus_client_get_batch_file_status (LnxdriveDbusClient  *self,
                                                                const char         **paths,
                                                                gsize                n_paths);

/* Asynchronous D-Bus actions. */
void                lnxdrive_dbus_client_pin_file   (LnxdriveDbusClient  *self,
                                                     const char          *path,
                                                     GAsyncReadyCallback  callback,
                                                     gpointer             user_data);
gboolean            lnxdrive_dbus_client_pin_file_finish (LnxdriveDbusClient  *self,
                                                          GAsyncResult        *result,
                                                          GError             **error);

void                lnxdrive_dbus_client_unpin_file (LnxdriveDbusClient  *self,
                                                     const char          *path,
                                                     GAsyncReadyCallback  callback,
                                                     gpointer             user_data);
gboolean            lnxdrive_dbus_client_unpin_file_finish (LnxdriveDbusClient  *self,
                                                            GAsyncResult        *result,
                                                            GError             **error);

void                lnxdrive_dbus_client_sync_path  (LnxdriveDbusClient  *self,
                                                     const char          *path,
                                                     GAsyncReadyCallback  callback,
                                                     gpointer             user_data);
gboolean            lnxdrive_dbus_client_sync_path_finish (LnxdriveDbusClient  *self,
                                                           GAsyncResult        *result,
                                                           GError             **error);

/* State queries. */
gboolean            lnxdrive_dbus_client_is_daemon_running (LnxdriveDbusClient *self);
const char         *lnxdrive_dbus_client_get_sync_root     (LnxdriveDbusClient *self);

/* Register a callback that the extension can use to trigger
 * nautilus_file_info_invalidate_extension_info() on all visible files. */
void                lnxdrive_dbus_client_set_invalidate_func (LnxdriveDbusClient   *self,
                                                              LnxdriveInvalidateFunc func,
                                                              gpointer               user_data);

G_END_DECLS

#endif /* LNXDRIVE_DBUS_CLIENT_H */
