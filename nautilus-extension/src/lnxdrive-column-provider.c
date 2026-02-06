/* lnxdrive-column-provider.c — NautilusColumnProvider for custom columns
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Provides two custom columns in Nautilus list view:
 *   1. "LNXDrive Status"  — textual sync status (Synced, Cloud Only, etc.)
 *   2. "Last Synced"      — human-readable timestamp of last sync
 *
 * Column data is populated by the InfoProvider via
 * nautilus_file_info_add_string_attribute().
 */

#include "lnxdrive-column-provider.h"

#include <nautilus-extension.h>
#include <glib/gi18n.h>

/* ---------------------------------------------------------------------------
 * Type definition
 * ---------------------------------------------------------------------------*/
struct _LnxdriveColumnProvider
{
    GObject parent_instance;
};

/* Forward declaration for interface init. */
static void lnxdrive_column_provider_iface_init (NautilusColumnProviderInterface *iface);

G_DEFINE_DYNAMIC_TYPE_EXTENDED (
    LnxdriveColumnProvider,
    lnxdrive_column_provider,
    G_TYPE_OBJECT,
    0,
    G_IMPLEMENT_INTERFACE_DYNAMIC (NAUTILUS_TYPE_COLUMN_PROVIDER,
                                   lnxdrive_column_provider_iface_init))

/* ---------------------------------------------------------------------------
 * NautilusColumnProvider interface implementation
 * ---------------------------------------------------------------------------*/
static GList *
lnxdrive_column_provider_get_columns (NautilusColumnProvider *provider)
{
    (void) provider;

    GList *columns = NULL;

    /* Column 1: LNXDrive sync status. */
    NautilusColumn *status_col = nautilus_column_new (
        "LNXDrive::status",           /* name (identifier)   */
        "LNXDrive::status",           /* attribute           */
        _("LNXDrive Status"),         /* label               */
        _("Sync status of the file in LNXDrive"));  /* description */

    columns = g_list_append (columns, status_col);

    /* Column 2: Last sync timestamp. */
    NautilusColumn *sync_col = nautilus_column_new (
        "LNXDrive::last_sync",        /* name (identifier)   */
        "LNXDrive::last_sync",        /* attribute           */
        _("Last Synced"),             /* label               */
        _("When the file was last synchronized"));  /* description */

    columns = g_list_append (columns, sync_col);

    return columns;
}

static void
lnxdrive_column_provider_iface_init (NautilusColumnProviderInterface *iface)
{
    iface->get_columns = lnxdrive_column_provider_get_columns;
}

/* ---------------------------------------------------------------------------
 * GObject boilerplate
 * ---------------------------------------------------------------------------*/
static void
lnxdrive_column_provider_class_init (LnxdriveColumnProviderClass *klass)
{
    (void) klass;
}

static void
lnxdrive_column_provider_class_finalize (LnxdriveColumnProviderClass *klass)
{
    (void) klass;
}

static void
lnxdrive_column_provider_init (LnxdriveColumnProvider *self)
{
    (void) self;
}

/* ---------------------------------------------------------------------------
 * Dynamic type registration (called from nautilus_module_initialize).
 *
 * G_DEFINE_DYNAMIC_TYPE_EXTENDED generates a static
 * lnxdrive_column_provider_register_type(). We expose a public wrapper
 * with a different name to avoid the static/extern linkage conflict.
 * ---------------------------------------------------------------------------*/
void
lnxdrive_column_provider_register (GTypeModule *module)
{
    lnxdrive_column_provider_register_type (module);
}
