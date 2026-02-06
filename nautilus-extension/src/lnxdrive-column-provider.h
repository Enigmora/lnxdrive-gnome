/* lnxdrive-column-provider.h — NautilusColumnProvider for LNXDrive custom columns
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#ifndef LNXDRIVE_COLUMN_PROVIDER_H
#define LNXDRIVE_COLUMN_PROVIDER_H

#include <glib-object.h>

G_BEGIN_DECLS

#define LNXDRIVE_TYPE_COLUMN_PROVIDER (lnxdrive_column_provider_get_type ())

G_DECLARE_FINAL_TYPE (LnxdriveColumnProvider, lnxdrive_column_provider,
                      LNXDRIVE, COLUMN_PROVIDER, GObject)

/* Register the type with a GTypeModule (required for Nautilus extension loading).
 * Called from nautilus_module_initialize(). */
void lnxdrive_column_provider_register (GTypeModule *module);

G_END_DECLS

#endif /* LNXDRIVE_COLUMN_PROVIDER_H */
