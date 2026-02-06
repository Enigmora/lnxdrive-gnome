/* lnxdrive-info-provider.h — NautilusInfoProvider for overlay icons and attributes
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#ifndef LNXDRIVE_INFO_PROVIDER_H
#define LNXDRIVE_INFO_PROVIDER_H

#include <glib-object.h>

G_BEGIN_DECLS

#define LNXDRIVE_TYPE_INFO_PROVIDER (lnxdrive_info_provider_get_type ())

G_DECLARE_FINAL_TYPE (LnxdriveInfoProvider, lnxdrive_info_provider,
                      LNXDRIVE, INFO_PROVIDER, GObject)

/* Register the type with a GTypeModule (required for Nautilus extension loading).
 * Called from nautilus_module_initialize(). */
void lnxdrive_info_provider_register (GTypeModule *module);

G_END_DECLS

#endif /* LNXDRIVE_INFO_PROVIDER_H */
