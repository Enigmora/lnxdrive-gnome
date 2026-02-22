/* lnxdrive-extension.c — Nautilus extension module entry point
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * This file provides the three entry points that Nautilus expects every
 * extension shared module to export:
 *
 *   nautilus_module_initialize()  — register GTypes with the GTypeModule
 *   nautilus_module_list_types()  — enumerate the GTypes we provide
 *   nautilus_module_shutdown()    — cleanup on unload
 *
 * The extension registers three provider types:
 *   1. LnxdriveInfoProvider   — overlay icons and string attributes
 *   2. LnxdriveMenuProvider   — context menu with Pin/Unpin/Sync actions
 *   3. LnxdriveColumnProvider — custom "LNXDrive Status" and "Last Synced" columns
 */

#include <nautilus-extension.h>
#include <glib/gi18n-lib.h>

#include "lnxdrive-dbus-client.h"
#include "lnxdrive-info-provider.h"
#include "lnxdrive-menu-provider.h"
#include "lnxdrive-column-provider.h"

/* The GTypes we register. Filled in by nautilus_module_initialize(). */
static GType provider_types[3];
static gint  n_provider_types = 0;

/* ---------------------------------------------------------------------------
 * Module entry points (exported symbols)
 * ---------------------------------------------------------------------------*/
void
nautilus_module_initialize (GTypeModule *module)
{
    /* Set up gettext for our translations. */
    bindtextdomain (GETTEXT_PACKAGE, LOCALEDIR);
    bind_textdomain_codeset (GETTEXT_PACKAGE, "UTF-8");

    /* Register all provider types with the GTypeModule. */
    lnxdrive_info_provider_register (module);
    lnxdrive_menu_provider_register (module);
    lnxdrive_column_provider_register (module);

    provider_types[0] = LNXDRIVE_TYPE_INFO_PROVIDER;
    provider_types[1] = LNXDRIVE_TYPE_MENU_PROVIDER;
    provider_types[2] = LNXDRIVE_TYPE_COLUMN_PROVIDER;
    n_provider_types  = G_N_ELEMENTS (provider_types);

    /* Initialize the D-Bus client singleton early so it can start
     * connecting asynchronously before Nautilus calls update_file_info().
     * File invalidation is handled directly by the info provider via the
     * "file-status-changed" GObject signal — no callback needed here. */
    lnxdrive_dbus_client_get_default ();

    g_info ("LNXDrive: Nautilus extension initialized (3 providers registered)");
}

void
nautilus_module_list_types (const GType **types,
                            int          *n_types)
{
    *types   = provider_types;
    *n_types = n_provider_types;
}

void
nautilus_module_shutdown (void)
{
    g_info ("LNXDrive: Nautilus extension shutting down");
    lnxdrive_dbus_client_release_default ();
}
