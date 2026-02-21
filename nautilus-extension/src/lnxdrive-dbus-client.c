/* lnxdrive-dbus-client.c — D-Bus client for communication with lnxdrive-daemon
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

#include "lnxdrive-dbus-client.h"

#include <gio/gio.h>
#include <glib/gi18n.h>
#include <string.h>

/* ---------------------------------------------------------------------------
 * Private data
 * ---------------------------------------------------------------------------*/
struct _LnxdriveDbusClient
{
    GObject parent_instance;

    GDBusProxy            *files_proxy;
    GHashTable            *status_cache;   /* (char *path -> char *status) */
    char                  *sync_root;
    gboolean               daemon_running;

    LnxdriveInvalidateFunc invalidate_cb;
    gpointer               invalidate_data;

    guint                  signal_subscription_id;
};

/* Signal IDs for the GObject signal "file-status-changed". */
enum
{
    SIGNAL_FILE_STATUS_CHANGED,
    N_SIGNALS
};

static guint signals[N_SIGNALS];

/* Singleton instance. */
static LnxdriveDbusClient *default_instance = NULL;

G_DEFINE_TYPE (LnxdriveDbusClient, lnxdrive_dbus_client, G_TYPE_OBJECT)

/* ---------------------------------------------------------------------------
 * Forward declarations
 * ---------------------------------------------------------------------------*/
static void on_proxy_ready            (GObject      *source,
                                       GAsyncResult *result,
                                       gpointer      user_data);
static void on_name_owner_changed     (GObject    *object,
                                       GParamSpec *pspec,
                                       gpointer    user_data);
static void on_file_status_changed    (GDBusConnection *connection,
                                       const gchar     *sender_name,
                                       const gchar     *object_path,
                                       const gchar     *interface_name,
                                       const gchar     *signal_name,
                                       GVariant        *parameters,
                                       gpointer         user_data);
static void fetch_sync_root_async     (LnxdriveDbusClient *self);
static void on_settings_proxy_ready   (GObject      *source,
                                       GAsyncResult *result,
                                       gpointer      user_data);
static void on_get_config_ready       (GObject      *source,
                                       GAsyncResult *result,
                                       gpointer      user_data);
static void on_action_call_ready      (GObject      *source,
                                       GAsyncResult *result,
                                       gpointer      user_data);

/* ---------------------------------------------------------------------------
 * Helpers
 * ---------------------------------------------------------------------------*/

/* Set every entry in the status cache to "unknown". Called when the daemon
 * disappears from the bus (FR-025: graceful degradation). */
static void
invalidate_all_cache_entries (LnxdriveDbusClient *self)
{
    GHashTableIter iter;
    gpointer       key;

    g_hash_table_iter_init (&iter, self->status_cache);
    while (g_hash_table_iter_next (&iter, &key, NULL))
    {
        g_hash_table_iter_replace (&iter, g_strdup ("unknown"));
    }
}

/* ---------------------------------------------------------------------------
 * GObject lifecycle
 * ---------------------------------------------------------------------------*/
static void
lnxdrive_dbus_client_constructed (GObject *object)
{
    LnxdriveDbusClient *self = LNXDRIVE_DBUS_CLIENT (object);

    G_OBJECT_CLASS (lnxdrive_dbus_client_parent_class)->constructed (object);

    /* Subscribe to the FileStatusChanged signal on the session bus.
     * We subscribe before the proxy is ready so we don't miss early signals. */
    GDBusConnection *bus = g_bus_get_sync (G_BUS_TYPE_SESSION, NULL, NULL);
    if (bus != NULL)
    {
        self->signal_subscription_id = g_dbus_connection_signal_subscribe (
            bus,
            LNXDRIVE_DBUS_BUS_NAME,               /* sender */
            LNXDRIVE_DBUS_IFACE_FILES,             /* interface */
            "FileStatusChanged",                    /* member */
            LNXDRIVE_DBUS_OBJECT_PATH,             /* object path */
            NULL,                                   /* arg0 */
            G_DBUS_SIGNAL_FLAGS_NONE,
            on_file_status_changed,
            self,
            NULL);
        g_object_unref (bus);
    }

    /* Create the GDBusProxy asynchronously. */
    g_dbus_proxy_new_for_bus (
        G_BUS_TYPE_SESSION,
        G_DBUS_PROXY_FLAGS_DO_NOT_AUTO_START,
        NULL,                                       /* GDBusInterfaceInfo */
        LNXDRIVE_DBUS_BUS_NAME,
        LNXDRIVE_DBUS_OBJECT_PATH,
        LNXDRIVE_DBUS_IFACE_FILES,
        NULL,                                       /* GCancellable */
        on_proxy_ready,
        self);
}

static void
lnxdrive_dbus_client_finalize (GObject *object)
{
    LnxdriveDbusClient *self = LNXDRIVE_DBUS_CLIENT (object);

    if (self->signal_subscription_id != 0)
    {
        GDBusConnection *bus = g_bus_get_sync (G_BUS_TYPE_SESSION, NULL, NULL);
        if (bus != NULL)
        {
            g_dbus_connection_signal_unsubscribe (bus, self->signal_subscription_id);
            g_object_unref (bus);
        }
        self->signal_subscription_id = 0;
    }

    g_clear_object (&self->files_proxy);
    g_clear_pointer (&self->status_cache, g_hash_table_unref);
    g_clear_pointer (&self->sync_root, g_free);

    G_OBJECT_CLASS (lnxdrive_dbus_client_parent_class)->finalize (object);
}

static void
lnxdrive_dbus_client_class_init (LnxdriveDbusClientClass *klass)
{
    GObjectClass *object_class = G_OBJECT_CLASS (klass);

    object_class->constructed = lnxdrive_dbus_client_constructed;
    object_class->finalize    = lnxdrive_dbus_client_finalize;

    /**
     * LnxdriveDbusClient::file-status-changed:
     * @self: the client instance
     * @path: absolute filesystem path whose status changed
     * @status: the new status string (e.g. "synced", "cloud-only")
     *
     * Emitted whenever the daemon reports a FileStatusChanged D-Bus signal.
     */
    signals[SIGNAL_FILE_STATUS_CHANGED] = g_signal_new (
        "file-status-changed",
        G_TYPE_FROM_CLASS (klass),
        G_SIGNAL_RUN_LAST,
        0,                              /* class offset */
        NULL, NULL,                     /* accumulator */
        NULL,                           /* C marshaller (use generic) */
        G_TYPE_NONE,
        2,
        G_TYPE_STRING,
        G_TYPE_STRING);
}

static void
lnxdrive_dbus_client_init (LnxdriveDbusClient *self)
{
    self->status_cache  = g_hash_table_new_full (g_str_hash, g_str_equal,
                                                  g_free, g_free);
    self->sync_root     = NULL;
    self->daemon_running = FALSE;
    self->files_proxy   = NULL;
    self->invalidate_cb = NULL;
    self->invalidate_data = NULL;
    self->signal_subscription_id = 0;
}

/* ---------------------------------------------------------------------------
 * Proxy ready callback
 * ---------------------------------------------------------------------------*/
static void
on_proxy_ready (GObject      *source,
                GAsyncResult *result,
                gpointer      user_data)
{
    LnxdriveDbusClient *self = LNXDRIVE_DBUS_CLIENT (user_data);
    g_autoptr (GError)  error = NULL;

    self->files_proxy = g_dbus_proxy_new_for_bus_finish (result, &error);
    if (self->files_proxy == NULL)
    {
        g_warning ("LNXDrive: failed to create D-Bus proxy for %s: %s",
                   LNXDRIVE_DBUS_IFACE_FILES,
                   error->message);
        return;
    }

    /* Watch for daemon appearing / disappearing. */
    g_signal_connect (self->files_proxy, "notify::g-name-owner",
                      G_CALLBACK (on_name_owner_changed), self);

    /* Check initial name owner. */
    g_autofree char *owner = g_dbus_proxy_get_name_owner (self->files_proxy);
    self->daemon_running = (owner != NULL);

    if (self->daemon_running)
    {
        fetch_sync_root_async (self);
    }

    g_debug ("LNXDrive: D-Bus proxy ready, daemon %s",
             self->daemon_running ? "running" : "not running");
}

/* ---------------------------------------------------------------------------
 * Name-owner tracking (FR-025: graceful degradation)
 * ---------------------------------------------------------------------------*/
static void
on_name_owner_changed (GObject    *object,
                       GParamSpec *pspec,
                       gpointer    user_data)
{
    LnxdriveDbusClient *self  = LNXDRIVE_DBUS_CLIENT (user_data);
    GDBusProxy         *proxy = G_DBUS_PROXY (object);

    g_autofree char *owner = g_dbus_proxy_get_name_owner (proxy);

    if (owner == NULL)
    {
        /* Daemon disappeared. */
        g_info ("LNXDrive: daemon has left the bus — entering degraded mode");
        self->daemon_running = FALSE;
        invalidate_all_cache_entries (self);

        /* Notify Nautilus to refresh its display. */
        if (self->invalidate_cb != NULL)
            self->invalidate_cb (self->invalidate_data);
    }
    else
    {
        /* Daemon (re-)appeared. */
        g_info ("LNXDrive: daemon appeared on the bus — re-querying state");
        self->daemon_running = TRUE;
        fetch_sync_root_async (self);

        /* Trigger re-display so emblems are updated from "unknown". */
        if (self->invalidate_cb != NULL)
            self->invalidate_cb (self->invalidate_data);
    }
}

/* ---------------------------------------------------------------------------
 * FileStatusChanged D-Bus signal handler
 * ---------------------------------------------------------------------------*/
static void
on_file_status_changed (GDBusConnection *connection,
                        const gchar     *sender_name,
                        const gchar     *object_path,
                        const gchar     *interface_name,
                        const gchar     *signal_name,
                        GVariant        *parameters,
                        gpointer         user_data)
{
    LnxdriveDbusClient *self = LNXDRIVE_DBUS_CLIENT (user_data);
    const char         *path   = NULL;
    const char         *status = NULL;

    g_variant_get (parameters, "(&s&s)", &path, &status);

    g_debug ("LNXDrive: FileStatusChanged(%s, %s)", path, status);

    /* Update local cache. */
    g_hash_table_replace (self->status_cache,
                          g_strdup (path),
                          g_strdup (status));

    /* Emit our GObject signal so providers can react. */
    g_signal_emit (self, signals[SIGNAL_FILE_STATUS_CHANGED], 0, path, status);

    /* Ask Nautilus to invalidate its display for affected files. */
    if (self->invalidate_cb != NULL)
        self->invalidate_cb (self->invalidate_data);
}

/* ---------------------------------------------------------------------------
 * Fetch sync root from the Settings interface (async fire-and-forget)
 * ---------------------------------------------------------------------------*/
static void
on_get_config_ready (GObject      *source,
                     GAsyncResult *result,
                     gpointer      user_data)
{
    LnxdriveDbusClient *self = LNXDRIVE_DBUS_CLIENT (user_data);
    GDBusProxy         *settings_proxy = G_DBUS_PROXY (source);
    g_autoptr (GError)   error = NULL;
    g_autoptr (GVariant) ret   = NULL;

    ret = g_dbus_proxy_call_finish (settings_proxy, result, &error);

    /* Release the settings proxy — we only needed it for this one call.
     * It was ref'd in on_settings_proxy_ready to survive until now. */
    g_object_unref (settings_proxy);

    if (ret == NULL)
    {
        g_warning ("LNXDrive: failed to get config: %s", error->message);
        /* Fall back to default sync root. */
        g_free (self->sync_root);
        self->sync_root = g_build_filename (g_get_home_dir (), "OneDrive", NULL);
        return;
    }

    const char *yaml_str = NULL;
    g_variant_get (ret, "(&s)", &yaml_str);

    /* Minimal YAML parsing: look for "sync_root:" line.
     * A proper YAML parser is overkill here; the value is always a simple path. */
    const char *key = "sync_root:";
    const char *pos = strstr (yaml_str, key);
    if (pos != NULL)
    {
        pos += strlen (key);
        /* Skip whitespace. */
        while (*pos == ' ' || *pos == '\t')
            pos++;

        /* Read until newline or end. */
        const char *end = pos;
        while (*end != '\0' && *end != '\n' && *end != '\r')
            end++;

        g_autofree char *raw = g_strndup (pos, (gsize)(end - pos));

        /* Expand ~ to home directory. */
        if (raw[0] == '~' && (raw[1] == '/' || raw[1] == '\0'))
        {
            g_free (self->sync_root);
            self->sync_root = g_build_filename (g_get_home_dir (),
                                                 raw + 2,
                                                 NULL);
        }
        else
        {
            g_free (self->sync_root);
            self->sync_root = g_steal_pointer (&raw);
        }
    }
    else
    {
        g_free (self->sync_root);
        self->sync_root = g_build_filename (g_get_home_dir (), "OneDrive", NULL);
    }

    g_info ("LNXDrive: sync root = %s", self->sync_root);
}

static void
on_settings_proxy_ready (GObject      *source,
                         GAsyncResult *result,
                         gpointer      user_data)
{
    LnxdriveDbusClient *self = LNXDRIVE_DBUS_CLIENT (user_data);
    g_autoptr (GError) error = NULL;

    GDBusProxy *settings_proxy =
        g_dbus_proxy_new_for_bus_finish (result, &error);

    if (settings_proxy == NULL)
    {
        g_warning ("LNXDrive: failed to create Settings proxy: %s", error->message);
        g_free (self->sync_root);
        self->sync_root = g_build_filename (g_get_home_dir (), "OneDrive", NULL);
        return;
    }

    /* Keep the proxy alive until on_get_config_ready releases it. */
    g_dbus_proxy_call (settings_proxy,
                       "GetConfig",
                       NULL,
                       G_DBUS_CALL_FLAGS_NONE,
                       5000,     /* 5 s timeout */
                       NULL,
                       on_get_config_ready,
                       self);
}

static void
fetch_sync_root_async (LnxdriveDbusClient *self)
{
    /* We call GetConfig on the Settings interface, which lives on the same
     * object path but a different interface. We create a quick proxy. */
    g_dbus_proxy_new_for_bus (
        G_BUS_TYPE_SESSION,
        G_DBUS_PROXY_FLAGS_DO_NOT_AUTO_START,
        NULL,
        LNXDRIVE_DBUS_BUS_NAME,
        LNXDRIVE_DBUS_OBJECT_PATH,
        "com.enigmora.LNXDrive.Settings",
        NULL,
        on_settings_proxy_ready,
        self);
}

/* ---------------------------------------------------------------------------
 * Shared trampoline for all void-returning D-Bus action calls
 * ---------------------------------------------------------------------------*/
static void
on_action_call_ready (GObject      *source,
                      GAsyncResult *result,
                      gpointer      user_data)
{
    GTask        *task  = G_TASK (user_data);
    GDBusProxy   *proxy = G_DBUS_PROXY (source);
    g_autoptr (GError) error = NULL;

    g_autoptr (GVariant) ret = g_dbus_proxy_call_finish (proxy, result, &error);

    if (ret == NULL)
        g_task_return_error (task, g_steal_pointer (&error));
    else
        g_task_return_boolean (task, TRUE);

    g_object_unref (task);
}

/* ---------------------------------------------------------------------------
 * Public API: singleton
 * ---------------------------------------------------------------------------*/
LnxdriveDbusClient *
lnxdrive_dbus_client_get_default (void)
{
    if (default_instance == NULL)
    {
        default_instance = g_object_new (LNXDRIVE_TYPE_DBUS_CLIENT, NULL);
    }

    return default_instance;
}

void
lnxdrive_dbus_client_release_default (void)
{
    g_clear_object (&default_instance);
}

/* ---------------------------------------------------------------------------
 * Public API: file status (cache lookup)
 * ---------------------------------------------------------------------------*/
const char *
lnxdrive_dbus_client_get_file_status (LnxdriveDbusClient *self,
                                      const char         *path)
{
    g_return_val_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self), "unknown");
    g_return_val_if_fail (path != NULL, "unknown");

    if (!self->daemon_running)
        return "unknown";

    const char *cached = g_hash_table_lookup (self->status_cache, path);
    return (cached != NULL) ? cached : "unknown";
}

/* ---------------------------------------------------------------------------
 * Public API: batch file status (synchronous D-Bus call)
 * ---------------------------------------------------------------------------*/
GHashTable *
lnxdrive_dbus_client_get_batch_file_status (LnxdriveDbusClient  *self,
                                            const char         **paths,
                                            gsize                n_paths)
{
    g_return_val_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self), NULL);

    GHashTable *result = g_hash_table_new_full (g_str_hash, g_str_equal,
                                                 g_free, g_free);

    if (!self->daemon_running || self->files_proxy == NULL || n_paths == 0)
        return result;

    /* Build the "as" variant for the paths array. */
    GVariantBuilder builder;
    g_variant_builder_init (&builder, G_VARIANT_TYPE ("as"));
    for (gsize i = 0; i < n_paths; i++)
        g_variant_builder_add (&builder, "s", paths[i]);

    g_autoptr (GError)   error = NULL;
    g_autoptr (GVariant) ret   = g_dbus_proxy_call_sync (
        self->files_proxy,
        "GetBatchFileStatus",
        g_variant_new ("(@as)", g_variant_builder_end (&builder)),
        G_DBUS_CALL_FLAGS_NONE,
        5000,     /* 5 s timeout */
        NULL,
        &error);

    if (ret == NULL)
    {
        g_warning ("LNXDrive: GetBatchFileStatus failed: %s", error->message);
        return result;
    }

    /* Parse "a{ss}" result. */
    g_autoptr (GVariant) dict = g_variant_get_child_value (ret, 0);
    GVariantIter iter;
    const char  *key;
    const char  *value;

    g_variant_iter_init (&iter, dict);
    while (g_variant_iter_next (&iter, "{&s&s}", &key, &value))
    {
        g_hash_table_replace (result, g_strdup (key), g_strdup (value));

        /* Update the local cache too. */
        g_hash_table_replace (self->status_cache,
                              g_strdup (key),
                              g_strdup (value));
    }

    return result;
}

/* ---------------------------------------------------------------------------
 * Public API: async actions — PinFile
 * ---------------------------------------------------------------------------*/
void
lnxdrive_dbus_client_pin_file (LnxdriveDbusClient  *self,
                               const char          *path,
                               GAsyncReadyCallback  callback,
                               gpointer             user_data)
{
    g_return_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self));
    g_return_if_fail (path != NULL);

    if (self->files_proxy == NULL)
    {
        g_task_report_new_error (self, callback, user_data,
                                lnxdrive_dbus_client_pin_file,
                                G_IO_ERROR, G_IO_ERROR_NOT_CONNECTED,
                                _("LNXDrive daemon is not available"));
        return;
    }

    GTask *task = g_task_new (self, NULL, callback, user_data);

    g_dbus_proxy_call (self->files_proxy,
                       "PinFile",
                       g_variant_new ("(s)", path),
                       G_DBUS_CALL_FLAGS_NONE,
                       30000,    /* 30 s — pinning may involve download */
                       g_task_get_cancellable (task),
                       on_action_call_ready,
                       task);
}

gboolean
lnxdrive_dbus_client_pin_file_finish (LnxdriveDbusClient  *self,
                                      GAsyncResult        *result,
                                      GError             **error)
{
    return g_task_propagate_boolean (G_TASK (result), error);
}

/* ---------------------------------------------------------------------------
 * Public API: async actions — UnpinFile
 * ---------------------------------------------------------------------------*/
void
lnxdrive_dbus_client_unpin_file (LnxdriveDbusClient  *self,
                                 const char          *path,
                                 GAsyncReadyCallback  callback,
                                 gpointer             user_data)
{
    g_return_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self));
    g_return_if_fail (path != NULL);

    if (self->files_proxy == NULL)
    {
        g_task_report_new_error (self, callback, user_data,
                                lnxdrive_dbus_client_unpin_file,
                                G_IO_ERROR, G_IO_ERROR_NOT_CONNECTED,
                                _("LNXDrive daemon is not available"));
        return;
    }

    GTask *task = g_task_new (self, NULL, callback, user_data);

    g_dbus_proxy_call (self->files_proxy,
                       "UnpinFile",
                       g_variant_new ("(s)", path),
                       G_DBUS_CALL_FLAGS_NONE,
                       30000,
                       g_task_get_cancellable (task),
                       on_action_call_ready,
                       task);
}

gboolean
lnxdrive_dbus_client_unpin_file_finish (LnxdriveDbusClient  *self,
                                        GAsyncResult        *result,
                                        GError             **error)
{
    return g_task_propagate_boolean (G_TASK (result), error);
}

/* ---------------------------------------------------------------------------
 * Public API: async actions — SyncPath
 * ---------------------------------------------------------------------------*/
void
lnxdrive_dbus_client_sync_path (LnxdriveDbusClient  *self,
                                const char          *path,
                                GAsyncReadyCallback  callback,
                                gpointer             user_data)
{
    g_return_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self));
    g_return_if_fail (path != NULL);

    if (self->files_proxy == NULL)
    {
        g_task_report_new_error (self, callback, user_data,
                                lnxdrive_dbus_client_sync_path,
                                G_IO_ERROR, G_IO_ERROR_NOT_CONNECTED,
                                _("LNXDrive daemon is not available"));
        return;
    }

    GTask *task = g_task_new (self, NULL, callback, user_data);

    g_dbus_proxy_call (self->files_proxy,
                       "SyncPath",
                       g_variant_new ("(s)", path),
                       G_DBUS_CALL_FLAGS_NONE,
                       30000,
                       g_task_get_cancellable (task),
                       on_action_call_ready,
                       task);
}

gboolean
lnxdrive_dbus_client_sync_path_finish (LnxdriveDbusClient  *self,
                                       GAsyncResult        *result,
                                       GError             **error)
{
    return g_task_propagate_boolean (G_TASK (result), error);
}

/* ---------------------------------------------------------------------------
 * Public API: state queries
 * ---------------------------------------------------------------------------*/
gboolean
lnxdrive_dbus_client_is_daemon_running (LnxdriveDbusClient *self)
{
    g_return_val_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self), FALSE);
    return self->daemon_running;
}

const char *
lnxdrive_dbus_client_get_sync_root (LnxdriveDbusClient *self)
{
    g_return_val_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self), NULL);
    return self->sync_root;
}

/* ---------------------------------------------------------------------------
 * Public API: invalidate callback
 * ---------------------------------------------------------------------------*/
void
lnxdrive_dbus_client_set_invalidate_func (LnxdriveDbusClient   *self,
                                          LnxdriveInvalidateFunc func,
                                          gpointer               user_data)
{
    g_return_if_fail (LNXDRIVE_IS_DBUS_CLIENT (self));

    self->invalidate_cb   = func;
    self->invalidate_data = user_data;
}
