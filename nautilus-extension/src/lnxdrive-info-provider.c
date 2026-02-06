/* lnxdrive-info-provider.c — NautilusInfoProvider for overlay icons (US1)
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Implements:
 *   - Emblem overlay icons showing sync status on files (FR-001..FR-004)
 *   - Custom string attributes for LNXDrive::status and LNXDrive::last_sync
 *   - Cache-first approach: returns from D-Bus client cache, triggers async
 *     batch queries for uncached entries
 *   - Invalidation via D-Bus FileStatusChanged signal (real-time updates)
 */

#include "lnxdrive-info-provider.h"
#include "lnxdrive-dbus-client.h"

#include <nautilus-extension.h>
#include <glib/gi18n.h>
#include <string.h>

/* ---------------------------------------------------------------------------
 * Type definition — we implement NautilusInfoProvider via an interface
 * ---------------------------------------------------------------------------*/
struct _LnxdriveInfoProvider
{
    GObject parent_instance;
};

/* Forward declarations for interface methods. */
static void lnxdrive_info_provider_iface_init (NautilusInfoProviderInterface *iface);

/* We use G_DEFINE_DYNAMIC_TYPE_EXTENDED so the type is registered with a
 * GTypeModule, which is required for Nautilus extension shared modules. */
G_DEFINE_DYNAMIC_TYPE_EXTENDED (
    LnxdriveInfoProvider,
    lnxdrive_info_provider,
    G_TYPE_OBJECT,
    0,
    G_IMPLEMENT_INTERFACE_DYNAMIC (NAUTILUS_TYPE_INFO_PROVIDER,
                                   lnxdrive_info_provider_iface_init))

/* ---------------------------------------------------------------------------
 * Status -> emblem mapping
 * ---------------------------------------------------------------------------*/

/* Map a D-Bus status string to an emblem icon name.
 * Returns NULL if no emblem should be applied (e.g. "excluded"). */
static const char *
status_to_emblem (const char *status)
{
    if (status == NULL)
        return "lnxdrive-unknown";

    if (g_str_equal (status, "synced"))
        return "lnxdrive-synced";
    if (g_str_equal (status, "cloud-only"))
        return "lnxdrive-cloud-only";
    if (g_str_equal (status, "syncing"))
        return "lnxdrive-syncing";
    if (g_str_equal (status, "pending"))
        return "lnxdrive-pending";
    if (g_str_equal (status, "conflict"))
        return "lnxdrive-conflict";
    if (g_str_equal (status, "error"))
        return "lnxdrive-error";
    if (g_str_equal (status, "unknown"))
        return "lnxdrive-unknown";

    /* "excluded" files: no emblem.
     * Pending issue I2: decide if excluded files should show a distinct
     * visual indicator. For now, we suppress the emblem entirely so the
     * file appears as an ordinary non-managed file. */
    if (g_str_equal (status, "excluded"))
        return NULL;

    /* Fallback for any unrecognized status string. */
    return "lnxdrive-unknown";
}

/* Map a D-Bus status string to a user-facing label for the column. */
static const char *
status_to_label (const char *status)
{
    if (status == NULL || g_str_equal (status, "unknown"))
        return _("Unknown");
    if (g_str_equal (status, "synced"))
        return _("Synced");
    if (g_str_equal (status, "cloud-only"))
        return _("Cloud Only");
    if (g_str_equal (status, "syncing"))
        return _("Syncing");
    if (g_str_equal (status, "pending"))
        return _("Pending");
    if (g_str_equal (status, "conflict"))
        return _("Conflict");
    if (g_str_equal (status, "error"))
        return _("Error");
    if (g_str_equal (status, "excluded"))
        return _("Excluded");

    return _("Unknown");
}

/* ---------------------------------------------------------------------------
 * URI -> local path helper
 * ---------------------------------------------------------------------------*/

/* Convert a Nautilus file URI to a local filesystem path.
 * Returns a newly allocated string, or NULL if the URI is not file://. */
static char *
uri_to_local_path (const char *uri)
{
    g_autoptr (GFile) file = g_file_new_for_uri (uri);
    return g_file_get_path (file);
}

/* ---------------------------------------------------------------------------
 * Check if a path is inside the sync root
 * ---------------------------------------------------------------------------*/
static gboolean
path_is_under_sync_root (const char *path, const char *sync_root)
{
    if (path == NULL || sync_root == NULL)
        return FALSE;

    gsize root_len = strlen (sync_root);
    if (root_len == 0)
        return FALSE;

    /* path must start with sync_root. */
    if (strncmp (path, sync_root, root_len) != 0)
        return FALSE;

    /* After the prefix we need either '\0' (the root itself) or '/'. */
    char c = path[root_len];
    return (c == '\0' || c == '/');
}

/* ---------------------------------------------------------------------------
 * NautilusInfoProvider interface implementation
 * ---------------------------------------------------------------------------*/
static NautilusOperationResult
lnxdrive_info_provider_update_file_info (NautilusInfoProvider     *provider,
                                         NautilusFileInfo         *file,
                                         GClosure                 *update_complete,
                                         NautilusOperationHandle **handle)
{
    (void) provider;
    (void) update_complete;
    (void) handle;

    /* Step 1: Get the local filesystem path from the file URI. */
    g_autofree char *uri  = nautilus_file_info_get_uri (file);
    g_autofree char *path = uri_to_local_path (uri);

    if (path == NULL)
        return NAUTILUS_OPERATION_COMPLETE;

    /* Step 2: Check if this file is under the sync root. */
    LnxdriveDbusClient *client    = lnxdrive_dbus_client_get_default ();
    const char         *sync_root = lnxdrive_dbus_client_get_sync_root (client);

    if (!path_is_under_sync_root (path, sync_root))
        return NAUTILUS_OPERATION_COMPLETE;

    /* Step 3: Query status from the D-Bus client cache. */
    const char *status = lnxdrive_dbus_client_get_file_status (client, path);

    /* Step 4: Map status to emblem and apply it. */
    const char *emblem = status_to_emblem (status);
    if (emblem != NULL)
        nautilus_file_info_add_emblem (file, emblem);

    /* Step 5: Set custom string attributes for the column provider. */
    const char *label = status_to_label (status);
    nautilus_file_info_add_string_attribute (file, "LNXDrive::status", label);

    /* For last_sync, we don't have per-file timestamps from the daemon yet.
     * Use a placeholder; the column will show "—" until the daemon provides
     * per-file sync timestamps in a future iteration. */
    nautilus_file_info_add_string_attribute (file, "LNXDrive::last_sync", "\xE2\x80\x94");

    /* Step 6: Return COMPLETE since we use the cache (synchronous).
     * If the cache did not contain the entry the status will be "unknown"
     * and will refresh once the daemon sends a FileStatusChanged signal. */
    return NAUTILUS_OPERATION_COMPLETE;
}

static void
lnxdrive_info_provider_cancel_update (NautilusInfoProvider    *provider,
                                      NautilusOperationHandle *handle)
{
    (void) provider;
    (void) handle;

    /* Currently all queries are synchronous cache lookups, so there is
     * nothing to cancel. If we add async batch queries in the future,
     * we would cancel the pending GCancellable here. */
}

static void
lnxdrive_info_provider_iface_init (NautilusInfoProviderInterface *iface)
{
    iface->update_file_info = lnxdrive_info_provider_update_file_info;
    iface->cancel_update    = lnxdrive_info_provider_cancel_update;
}

/* ---------------------------------------------------------------------------
 * GObject boilerplate
 * ---------------------------------------------------------------------------*/
static void
lnxdrive_info_provider_class_init (LnxdriveInfoProviderClass *klass)
{
    (void) klass;
}

static void
lnxdrive_info_provider_class_finalize (LnxdriveInfoProviderClass *klass)
{
    (void) klass;
}

static void
lnxdrive_info_provider_init (LnxdriveInfoProvider *self)
{
    (void) self;
}

/* ---------------------------------------------------------------------------
 * Dynamic type registration (called from nautilus_module_initialize).
 *
 * G_DEFINE_DYNAMIC_TYPE_EXTENDED generates a static
 * lnxdrive_info_provider_register_type(). We expose a public wrapper
 * with a different name to avoid the static/extern linkage conflict.
 * ---------------------------------------------------------------------------*/
void
lnxdrive_info_provider_register (GTypeModule *module)
{
    lnxdrive_info_provider_register_type (module);
}
