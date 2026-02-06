/* lnxdrive-menu-provider.c — NautilusMenuProvider for context-menu actions (US2)
 *
 * Copyright 2026 Enigmora <https://enigmora.com>
 * SPDX-License-Identifier: GPL-3.0-or-later
 *
 * Implements:
 *   - "Keep Available Offline"  (Pin)   for cloud-only files  (FR-006)
 *   - "Free Up Space"           (Unpin) for locally pinned    (FR-006)
 *   - "Sync Now"                        for any managed file  (FR-006)
 *   - Multi-selection support                                 (FR-007)
 *   - Disabled menu when daemon is offline                    (FR-025)
 *   - Background item: "Sync This Folder"
 *
 * Error handling (T039):
 *   - InsufficientDiskSpace  -> GNotification
 *   - FileInUse              -> GNotification
 *   - InvalidPath            -> GNotification
 *   - Generic GError         -> GNotification
 */

#include "lnxdrive-menu-provider.h"
#include "lnxdrive-dbus-client.h"

#include <nautilus-extension.h>
#include <glib/gi18n.h>
#include <string.h>

/* ---------------------------------------------------------------------------
 * Type definition
 * ---------------------------------------------------------------------------*/
struct _LnxdriveMenuProvider
{
    GObject parent_instance;
};

/* Forward declarations. */
static void lnxdrive_menu_provider_iface_init (NautilusMenuProviderInterface *iface);

G_DEFINE_DYNAMIC_TYPE_EXTENDED (
    LnxdriveMenuProvider,
    lnxdrive_menu_provider,
    G_TYPE_OBJECT,
    0,
    G_IMPLEMENT_INTERFACE_DYNAMIC (NAUTILUS_TYPE_MENU_PROVIDER,
                                   lnxdrive_menu_provider_iface_init))

/* ---------------------------------------------------------------------------
 * Helpers
 * ---------------------------------------------------------------------------*/

/* Convert a Nautilus file URI to a local filesystem path.
 * Returns a newly allocated string, or NULL if the URI is not file://. */
static char *
uri_to_local_path (const char *uri)
{
    g_autoptr (GFile) file = g_file_new_for_uri (uri);
    return g_file_get_path (file);
}

/* Check if a path is inside the sync root. */
static gboolean
path_is_under_sync_root (const char *path, const char *sync_root)
{
    if (path == NULL || sync_root == NULL)
        return FALSE;

    gsize root_len = strlen (sync_root);
    if (root_len == 0)
        return FALSE;

    if (strncmp (path, sync_root, root_len) != 0)
        return FALSE;

    char c = path[root_len];
    return (c == '\0' || c == '/');
}

/* ---------------------------------------------------------------------------
 * Error notification (T039)
 * ---------------------------------------------------------------------------*/

/* Show a desktop notification for an operation error. */
static void
show_error_notification (const char *title, const char *body)
{
    g_autoptr (GNotification) notification = g_notification_new (title);
    g_notification_set_body (notification, body);
    g_notification_set_priority (notification, G_NOTIFICATION_PRIORITY_NORMAL);

    GApplication *app = g_application_get_default ();  /* transfer-none */
    if (app != NULL)
    {
        g_application_send_notification (app, "lnxdrive-action-error", notification);
    }
    else
    {
        /* If there is no GApplication (common in Nautilus extensions), fall
         * back to g_warning so the error is not silently lost. */
        g_warning ("LNXDrive: %s — %s", title, body);
    }
}

/* Classify and report a D-Bus error after a Pin/Unpin/Sync action. */
static void
handle_action_error (GError *error, const char *action_name)
{
    if (error == NULL)
        return;

    const char *dbus_error = g_dbus_error_get_remote_error (error);

    if (g_strcmp0 (dbus_error, LNXDRIVE_DBUS_ERROR_INSUFFICIENT_DISK_SPACE) == 0)
    {
        show_error_notification (
            _("Not Enough Disk Space"),
            _("There is not enough disk space to complete this operation. "
              "Free up some space and try again."));
    }
    else if (g_strcmp0 (dbus_error, LNXDRIVE_DBUS_ERROR_FILE_IN_USE) == 0)
    {
        show_error_notification (
            _("File In Use"),
            _("The file is currently in use by another process. "
              "Close the file and try again."));
    }
    else if (g_strcmp0 (dbus_error, LNXDRIVE_DBUS_ERROR_INVALID_PATH) == 0)
    {
        show_error_notification (
            _("File Not in Sync Folder"),
            _("This file is not inside the LNXDrive sync folder."));
    }
    else
    {
        g_autofree char *msg =
            g_strdup_printf (_("The \"%s\" operation failed: %s"),
                             action_name, error->message);
        show_error_notification (_("LNXDrive: Operation Failed"), msg);
    }
}

/* ---------------------------------------------------------------------------
 * Async action callbacks
 * ---------------------------------------------------------------------------*/

static void
on_pin_file_done (GObject      *source,
                  GAsyncResult *result,
                  gpointer      user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = LNXDRIVE_DBUS_CLIENT (source);
    g_autoptr (GError) error = NULL;

    if (!lnxdrive_dbus_client_pin_file_finish (client, result, &error))
        handle_action_error (error, _("Keep Available Offline"));
}

static void
on_unpin_file_done (GObject      *source,
                    GAsyncResult *result,
                    gpointer      user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = LNXDRIVE_DBUS_CLIENT (source);
    g_autoptr (GError) error = NULL;

    if (!lnxdrive_dbus_client_unpin_file_finish (client, result, &error))
        handle_action_error (error, _("Free Up Space"));
}

static void
on_sync_path_done (GObject      *source,
                   GAsyncResult *result,
                   gpointer      user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = LNXDRIVE_DBUS_CLIENT (source);
    g_autoptr (GError) error = NULL;

    if (!lnxdrive_dbus_client_sync_path_finish (client, result, &error))
        handle_action_error (error, _("Sync Now"));
}

/* ---------------------------------------------------------------------------
 * Menu action signal handlers
 * ---------------------------------------------------------------------------*/

/* "activate" handler for "Keep Available Offline" (pin cloud-only files). */
static void
on_pin_activated (NautilusMenuItem *item,
                  gpointer          user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = lnxdrive_dbus_client_get_default ();

    GList *files = g_object_get_data (G_OBJECT (item), "lnxdrive-files");
    for (GList *l = files; l != NULL; l = l->next)
    {
        NautilusFileInfo *file_info = NAUTILUS_FILE_INFO (l->data);
        g_autofree char *uri  = nautilus_file_info_get_uri (file_info);
        g_autofree char *path = uri_to_local_path (uri);
        if (path == NULL)
            continue;

        const char *status = lnxdrive_dbus_client_get_file_status (client, path);
        if (g_str_equal (status, "cloud-only"))
        {
            lnxdrive_dbus_client_pin_file (client, path, on_pin_file_done, NULL);
        }
    }
}

/* "activate" handler for "Free Up Space" (unpin locally pinned files). */
static void
on_unpin_activated (NautilusMenuItem *item,
                   gpointer          user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = lnxdrive_dbus_client_get_default ();

    GList *files = g_object_get_data (G_OBJECT (item), "lnxdrive-files");
    for (GList *l = files; l != NULL; l = l->next)
    {
        NautilusFileInfo *file_info = NAUTILUS_FILE_INFO (l->data);
        g_autofree char *uri  = nautilus_file_info_get_uri (file_info);
        g_autofree char *path = uri_to_local_path (uri);
        if (path == NULL)
            continue;

        const char *status = lnxdrive_dbus_client_get_file_status (client, path);
        if (g_str_equal (status, "synced"))
        {
            lnxdrive_dbus_client_unpin_file (client, path, on_unpin_file_done, NULL);
        }
    }
}

/* "activate" handler for "Sync Now". */
static void
on_sync_activated (NautilusMenuItem *item,
                   gpointer          user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = lnxdrive_dbus_client_get_default ();

    GList *files = g_object_get_data (G_OBJECT (item), "lnxdrive-files");
    for (GList *l = files; l != NULL; l = l->next)
    {
        NautilusFileInfo *file_info = NAUTILUS_FILE_INFO (l->data);
        g_autofree char *uri  = nautilus_file_info_get_uri (file_info);
        g_autofree char *path = uri_to_local_path (uri);
        if (path == NULL)
            continue;

        lnxdrive_dbus_client_sync_path (client, path, on_sync_path_done, NULL);
    }
}

/* "activate" handler for background "Sync This Folder". */
static void
on_sync_folder_activated (NautilusMenuItem *item,
                          gpointer          user_data)
{
    (void) user_data;
    LnxdriveDbusClient *client = lnxdrive_dbus_client_get_default ();

    const char *folder_path = g_object_get_data (G_OBJECT (item), "lnxdrive-folder-path");
    if (folder_path != NULL)
    {
        lnxdrive_dbus_client_sync_path (client, folder_path, on_sync_path_done, NULL);
    }
}

/* ---------------------------------------------------------------------------
 * Helper: free a list of NautilusFileInfo objects that we ref'd.
 * ---------------------------------------------------------------------------*/
static void
file_info_list_free (gpointer data)
{
    GList *list = data;
    g_list_free_full (list, g_object_unref);
}

/* ---------------------------------------------------------------------------
 * Helper: attach file list to a menu item so callbacks can find it.
 * We store a shallow copy of the GList; the NautilusFileInfo objects
 * are owned by Nautilus and valid for the lifetime of the menu.
 * ---------------------------------------------------------------------------*/
static void
attach_files_to_item (NautilusMenuItem *item, GList *files)
{
    /* g_list_copy does a shallow copy; we ref each NautilusFileInfo so
     * the menu item callbacks can safely access them. */
    GList *copy = g_list_copy (files);
    for (GList *l = copy; l != NULL; l = l->next)
        g_object_ref (l->data);

    g_object_set_data_full (G_OBJECT (item), "lnxdrive-files", copy,
                            file_info_list_free);
}

/* ---------------------------------------------------------------------------
 * NautilusMenuProvider: get_file_items
 * ---------------------------------------------------------------------------*/
static GList *
lnxdrive_menu_provider_get_file_items (NautilusMenuProvider *provider,
                                       GList                *files)
{
    (void) provider;

    if (files == NULL)
        return NULL;

    LnxdriveDbusClient *client    = lnxdrive_dbus_client_get_default ();
    const char         *sync_root = lnxdrive_dbus_client_get_sync_root (client);

    /* ----- Daemon not running: show disabled indicator (FR-025) ----- */
    if (!lnxdrive_dbus_client_is_daemon_running (client))
    {
        NautilusMenuItem *disabled_item = nautilus_menu_item_new (
            "LNXDrive::service_unavailable",
            _("LNXDrive \xE2\x80\x94 Service Not Running"),
            _("The LNXDrive synchronization service is not running"),
            NULL);  /* no icon */

        g_object_set (disabled_item, "sensitive", FALSE, NULL);

        return g_list_append (NULL, disabled_item);
    }

    /* ----- Check if ANY selected file is under the sync root (FR-005) ----- */
    gboolean any_in_sync_root = FALSE;
    gboolean has_cloud_only   = FALSE;
    gboolean has_pinned       = FALSE;

    for (GList *l = files; l != NULL; l = l->next)
    {
        NautilusFileInfo *file_info = NAUTILUS_FILE_INFO (l->data);
        g_autofree char  *uri  = nautilus_file_info_get_uri (file_info);
        g_autofree char  *path = uri_to_local_path (uri);

        if (path == NULL)
            continue;

        if (!path_is_under_sync_root (path, sync_root))
            continue;

        any_in_sync_root = TRUE;

        const char *status = lnxdrive_dbus_client_get_file_status (client, path);

        if (g_str_equal (status, "cloud-only"))
            has_cloud_only = TRUE;
        else if (g_str_equal (status, "synced"))
            has_pinned = TRUE;
    }

    /* Nothing to show if no selected file is managed by LNXDrive (FR-005). */
    if (!any_in_sync_root)
        return NULL;

    /* ----- Build the top-level "LNXDrive" parent menu item + submenu ----- */
    NautilusMenuItem *top_item = nautilus_menu_item_new (
        "LNXDrive::top_menu",
        "LNXDrive",
        _("LNXDrive file actions"),
        "lnxdrive-synced");  /* icon name */

    NautilusMenu *submenu = nautilus_menu_new ();
    nautilus_menu_item_set_submenu (top_item, submenu);

    /* ----- Submenu items ----- */

    /* "Keep Available Offline" — only if there are cloud-only files (FR-006). */
    if (has_cloud_only)
    {
        NautilusMenuItem *pin_item = nautilus_menu_item_new (
            "LNXDrive::pin",
            _("Keep Available Offline"),
            _("Download selected cloud-only files and keep them available offline"),
            "folder-download-symbolic");

        attach_files_to_item (pin_item, files);
        g_signal_connect (pin_item, "activate",
                          G_CALLBACK (on_pin_activated), NULL);

        nautilus_menu_append_item (submenu, pin_item);
        g_object_unref (pin_item);
    }

    /* "Free Up Space" — only if there are locally pinned files (FR-006). */
    if (has_pinned)
    {
        NautilusMenuItem *unpin_item = nautilus_menu_item_new (
            "LNXDrive::unpin",
            _("Free Up Space"),
            _("Convert selected files to cloud-only placeholders to free disk space"),
            "edit-clear-symbolic");

        attach_files_to_item (unpin_item, files);
        g_signal_connect (unpin_item, "activate",
                          G_CALLBACK (on_unpin_activated), NULL);

        nautilus_menu_append_item (submenu, unpin_item);
        g_object_unref (unpin_item);
    }

    /* "Sync Now" — always available for managed files (FR-006). */
    {
        NautilusMenuItem *sync_item = nautilus_menu_item_new (
            "LNXDrive::sync_now",
            _("Sync Now"),
            _("Immediately synchronize selected files"),
            "emblem-synchronizing-symbolic");

        attach_files_to_item (sync_item, files);
        g_signal_connect (sync_item, "activate",
                          G_CALLBACK (on_sync_activated), NULL);

        nautilus_menu_append_item (submenu, sync_item);
        g_object_unref (sync_item);
    }

    g_object_unref (submenu);

    return g_list_append (NULL, top_item);
}

/* ---------------------------------------------------------------------------
 * NautilusMenuProvider: get_background_items
 * ---------------------------------------------------------------------------*/
static GList *
lnxdrive_menu_provider_get_background_items (NautilusMenuProvider *provider,
                                             NautilusFileInfo     *current_folder)
{
    (void) provider;

    if (current_folder == NULL)
        return NULL;

    LnxdriveDbusClient *client    = lnxdrive_dbus_client_get_default ();
    const char         *sync_root = lnxdrive_dbus_client_get_sync_root (client);

    if (!lnxdrive_dbus_client_is_daemon_running (client))
        return NULL;

    g_autofree char *uri         = nautilus_file_info_get_uri (current_folder);
    g_autofree char *folder_path = uri_to_local_path (uri);

    if (folder_path == NULL)
        return NULL;

    if (!path_is_under_sync_root (folder_path, sync_root))
        return NULL;

    /* Build "LNXDrive > Sync This Folder" for the background menu. */
    NautilusMenuItem *top_item = nautilus_menu_item_new (
        "LNXDrive::bg_top_menu",
        "LNXDrive",
        _("LNXDrive folder actions"),
        "lnxdrive-synced");

    NautilusMenu *submenu = nautilus_menu_new ();
    nautilus_menu_item_set_submenu (top_item, submenu);

    NautilusMenuItem *sync_item = nautilus_menu_item_new (
        "LNXDrive::sync_folder",
        _("Sync This Folder"),
        _("Immediately synchronize this folder"),
        "emblem-synchronizing-symbolic");

    /* Store the folder path so the callback can use it. */
    g_object_set_data_full (G_OBJECT (sync_item), "lnxdrive-folder-path",
                            g_strdup (folder_path), g_free);
    g_signal_connect (sync_item, "activate",
                      G_CALLBACK (on_sync_folder_activated), NULL);

    nautilus_menu_append_item (submenu, sync_item);

    g_object_unref (sync_item);
    g_object_unref (submenu);

    return g_list_append (NULL, top_item);
}

static void
lnxdrive_menu_provider_iface_init (NautilusMenuProviderInterface *iface)
{
    iface->get_file_items       = lnxdrive_menu_provider_get_file_items;
    iface->get_background_items = lnxdrive_menu_provider_get_background_items;
}

/* ---------------------------------------------------------------------------
 * GObject boilerplate
 * ---------------------------------------------------------------------------*/
static void
lnxdrive_menu_provider_class_init (LnxdriveMenuProviderClass *klass)
{
    (void) klass;
}

static void
lnxdrive_menu_provider_class_finalize (LnxdriveMenuProviderClass *klass)
{
    (void) klass;
}

static void
lnxdrive_menu_provider_init (LnxdriveMenuProvider *self)
{
    (void) self;
}

/* ---------------------------------------------------------------------------
 * Dynamic type registration (called from nautilus_module_initialize).
 *
 * G_DEFINE_DYNAMIC_TYPE_EXTENDED generates a static
 * lnxdrive_menu_provider_register_type(). We expose a public wrapper
 * with a different name to avoid the static/extern linkage conflict.
 * ---------------------------------------------------------------------------*/
void
lnxdrive_menu_provider_register (GTypeModule *module)
{
    lnxdrive_menu_provider_register_type (module);
}
