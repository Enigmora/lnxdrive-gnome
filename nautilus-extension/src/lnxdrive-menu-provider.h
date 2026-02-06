/* lnxdrive-menu-provider.h — NautilusMenuProvider for context-menu actions
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

#ifndef LNXDRIVE_MENU_PROVIDER_H
#define LNXDRIVE_MENU_PROVIDER_H

#include <glib-object.h>

G_BEGIN_DECLS

#define LNXDRIVE_TYPE_MENU_PROVIDER (lnxdrive_menu_provider_get_type ())

G_DECLARE_FINAL_TYPE (LnxdriveMenuProvider, lnxdrive_menu_provider,
                      LNXDRIVE, MENU_PROVIDER, GObject)

/* Register the type with a GTypeModule (required for Nautilus extension loading).
 * Called from nautilus_module_initialize(). */
void lnxdrive_menu_provider_register (GTypeModule *module);

G_END_DECLS

#endif /* LNXDRIVE_MENU_PROVIDER_H */
