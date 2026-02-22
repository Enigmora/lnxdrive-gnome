// LNXDrive D-Bus Client
//
// Provides a high-level async wrapper around the LNXDrive daemon's D-Bus interfaces.
// Uses zbus with the default async-io runtime so that futures are compatible with
// glib::MainContext::spawn_local() — do NOT use tokio for D-Bus operations.
//
// D-Bus coordinates:
//   Bus name:    com.enigmora.LNXDrive
//   Object path: /com/enigmora/LNXDrive
//
// Auth flow:
//   The daemon runs a loopback HTTP server for OAuth2 redirect (RFC 8252).
//   The app calls StartAuth() which returns (auth_url, state), then opens the
//   user's default browser at auth_url.  The daemon's loopback server captures
//   the redirect, exchanges the code for tokens, and emits the AuthStateChanged
//   D-Bus signal.  CompleteAuth(code, state) exists for manual, CLI, or
//   GNOME Online Accounts flows where the app provides the auth code directly.
//
// Terminology Glossary:
//   - CloudOnly = 'cloud-only' (D-Bus string) = 'placeholder' (user-facing term)
//   - UnpinFile = unpin + dehydrate (makes file cloud-only, frees local space)
//   - PinFile   = hydrate + pin (downloads file, keeps local)

use std::collections::HashMap;
use std::fmt;

use zbus::zvariant::OwnedValue;
use zbus::{proxy, Connection};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur when communicating with the LNXDrive daemon over D-Bus.
#[derive(Debug)]
pub enum DbusError {
    /// The D-Bus session bus could not be connected to, or a method call failed.
    Zbus(zbus::Error),
    /// The daemon returned an application-level error message.
    Daemon(String),
}

impl fmt::Display for DbusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zbus(e) => write!(f, "D-Bus error: {e}"),
            Self::Daemon(msg) => write!(f, "Daemon error: {msg}"),
        }
    }
}

impl std::error::Error for DbusError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Zbus(e) => Some(e),
            Self::Daemon(_) => None,
        }
    }
}

impl From<zbus::Error> for DbusError {
    fn from(e: zbus::Error) -> Self {
        Self::Zbus(e)
    }
}

// ---------------------------------------------------------------------------
// D-Bus proxy traits (generated via the #[proxy] macro)
// ---------------------------------------------------------------------------

/// com.enigmora.LNXDrive.Auth — authentication management
#[proxy(
    interface = "com.enigmora.LNXDrive.Auth",
    default_service = "com.enigmora.LNXDrive",
    default_path = "/com/enigmora/LNXDrive"
)]
pub trait LnxdriveAuth {
    /// Returns true if the user is currently authenticated.
    async fn is_authenticated(&self) -> zbus::Result<bool>;

    /// Begin OAuth2 flow. Returns (auth_url, state).
    async fn start_auth(&self) -> zbus::Result<(String, String)>;

    /// Finish an auth flow with an explicit code + state (manual/CLI/GOA).
    async fn complete_auth(&self, code: &str, state: &str) -> zbus::Result<bool>;

    /// Log out the current user and revoke tokens.
    async fn logout(&self) -> zbus::Result<()>;

    /// Emitted when the authentication state changes.
    /// The argument is the new state string, e.g. "authenticated", "unauthenticated", "error".
    #[zbus(signal)]
    fn auth_state_changed(&self, state: &str) -> zbus::Result<()>;
}

/// com.enigmora.LNXDrive.Settings — configuration and folder management
#[proxy(
    interface = "com.enigmora.LNXDrive.Settings",
    default_service = "com.enigmora.LNXDrive",
    default_path = "/com/enigmora/LNXDrive"
)]
trait LnxdriveSettings {
    /// Return the full configuration as a YAML string.
    async fn get_config(&self) -> zbus::Result<String>;

    /// Replace the full configuration with the supplied YAML string.
    async fn set_config(&self, yaml: &str) -> zbus::Result<()>;

    /// Return the list of folder paths selected for sync.
    async fn get_selected_folders(&self) -> zbus::Result<Vec<String>>;

    /// Set the list of folder paths selected for sync.
    async fn set_selected_folders(&self, folders: &[String]) -> zbus::Result<()>;

    /// Return the list of exclusion glob patterns.
    async fn get_exclusion_patterns(&self) -> zbus::Result<Vec<String>>;

    /// Set the list of exclusion glob patterns.
    async fn set_exclusion_patterns(&self, patterns: &[String]) -> zbus::Result<()>;

    /// Return the remote folder tree as a JSON string.
    async fn get_remote_folder_tree(&self) -> zbus::Result<String>;
}

/// com.enigmora.LNXDrive.Status — account and quota information
#[proxy(
    interface = "com.enigmora.LNXDrive.Status",
    default_service = "com.enigmora.LNXDrive",
    default_path = "/com/enigmora/LNXDrive"
)]
trait LnxdriveStatus {
    /// Return (used_bytes, total_bytes).
    async fn get_quota(&self) -> zbus::Result<(u64, u64)>;

    /// Return a dict of account metadata (display_name, email, etc.).
    async fn get_account_info(&self) -> zbus::Result<HashMap<String, OwnedValue>>;
}

/// com.enigmora.LNXDrive.Sync — sync control
#[proxy(
    interface = "com.enigmora.LNXDrive.Sync",
    default_service = "com.enigmora.LNXDrive",
    default_path = "/com/enigmora/LNXDrive"
)]
trait LnxdriveSync {
    /// Trigger an immediate sync cycle.
    async fn sync_now(&self) -> zbus::Result<()>;

    /// Pause sync.
    async fn pause(&self) -> zbus::Result<()>;

    /// Resume sync.
    async fn resume(&self) -> zbus::Result<()>;
}

/// com.enigmora.LNXDrive.Conflicts — conflict detection and resolution
#[proxy(
    interface = "com.enigmora.LNXDrive.Conflicts",
    default_service = "com.enigmora.LNXDrive",
    default_path = "/com/enigmora/LNXDrive"
)]
pub trait LnxdriveConflicts {
    /// List all unresolved conflicts as a JSON array.
    async fn list(&self) -> zbus::Result<String>;

    /// Get details for a specific conflict by ID. Returns JSON.
    async fn get_details(&self, id: &str) -> zbus::Result<String>;

    /// Resolve a conflict with the given strategy ("keep_local", "keep_remote", "keep_both").
    /// Returns true on success.
    async fn resolve(&self, id: &str, strategy: &str) -> zbus::Result<bool>;

    /// Resolve all unresolved conflicts with the given strategy.
    /// Returns the number of conflicts resolved.
    async fn resolve_all(&self, strategy: &str) -> zbus::Result<u32>;

    /// Emitted when a new conflict is detected.
    #[zbus(signal)]
    fn conflict_detected(&self, conflict_json: &str) -> zbus::Result<()>;

    /// Emitted when a conflict is resolved.
    #[zbus(signal)]
    fn conflict_resolved(&self, conflict_id: &str, strategy: &str) -> zbus::Result<()>;
}

// ---------------------------------------------------------------------------
// High-level client
// ---------------------------------------------------------------------------

/// A convenience wrapper that holds a D-Bus connection and exposes typed async
/// methods for every daemon operation.
#[derive(Clone)]
pub struct DbusClient {
    connection: Connection,
}

impl DbusClient {
    /// Connect to the session bus. This is async and should be spawned on the
    /// glib MainContext (e.g. via `glib::MainContext::default().spawn_local()`).
    pub async fn new() -> Result<Self, DbusError> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }

    // -- Auth ---------------------------------------------------------------

    /// Check whether the user is currently authenticated with the daemon.
    pub async fn is_authenticated(&self) -> Result<bool, DbusError> {
        let proxy = LnxdriveAuthProxy::new(&self.connection).await?;
        Ok(proxy.is_authenticated().await?)
    }

    /// Start the OAuth2 flow. Returns `(auth_url, state)`.
    /// The caller should open `auth_url` in the default browser.
    pub async fn start_auth(&self) -> Result<(String, String), DbusError> {
        let proxy = LnxdriveAuthProxy::new(&self.connection).await?;
        Ok(proxy.start_auth().await?)
    }

    /// Complete an auth flow manually (used by CLI or GOA integration).
    pub async fn complete_auth(&self, code: &str, state: &str) -> Result<bool, DbusError> {
        let proxy = LnxdriveAuthProxy::new(&self.connection).await?;
        Ok(proxy.complete_auth(code, state).await?)
    }

    /// Log out the current user.
    pub async fn logout(&self) -> Result<(), DbusError> {
        let proxy = LnxdriveAuthProxy::new(&self.connection).await?;
        Ok(proxy.logout().await?)
    }

    /// Get a clone of the underlying D-Bus connection.
    /// This can be used to create proxies for signal subscriptions, e.g.:
    /// ```ignore
    /// let proxy = LnxdriveAuthProxy::new(client.connection()).await?;
    /// let mut stream = proxy.receive_auth_state_changed().await?;
    /// ```
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    // -- Settings -----------------------------------------------------------

    /// Return the full configuration as YAML.
    pub async fn get_config(&self) -> Result<String, DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.get_config().await?)
    }

    /// Replace the configuration with the given YAML string.
    pub async fn set_config(&self, yaml: &str) -> Result<(), DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.set_config(yaml).await?)
    }

    /// Get the list of folders selected for sync.
    pub async fn get_selected_folders(&self) -> Result<Vec<String>, DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.get_selected_folders().await?)
    }

    /// Set the list of folders selected for sync.
    pub async fn set_selected_folders(&self, folders: &[String]) -> Result<(), DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.set_selected_folders(folders).await?)
    }

    /// Get the list of exclusion glob patterns.
    pub async fn get_exclusion_patterns(&self) -> Result<Vec<String>, DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.get_exclusion_patterns().await?)
    }

    /// Set the list of exclusion glob patterns.
    pub async fn set_exclusion_patterns(&self, patterns: &[String]) -> Result<(), DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.set_exclusion_patterns(patterns).await?)
    }

    /// Return the remote folder tree as a JSON string.
    pub async fn get_remote_folder_tree(&self) -> Result<String, DbusError> {
        let proxy = LnxdriveSettingsProxy::new(&self.connection).await?;
        Ok(proxy.get_remote_folder_tree().await?)
    }

    // -- Status -------------------------------------------------------------

    /// Return `(used_bytes, total_bytes)` quota.
    pub async fn get_quota(&self) -> Result<(u64, u64), DbusError> {
        let proxy = LnxdriveStatusProxy::new(&self.connection).await?;
        Ok(proxy.get_quota().await?)
    }

    /// Return account metadata as key-value pairs.
    pub async fn get_account_info(
        &self,
    ) -> Result<HashMap<String, OwnedValue>, DbusError> {
        let proxy = LnxdriveStatusProxy::new(&self.connection).await?;
        Ok(proxy.get_account_info().await?)
    }

    // -- Sync ---------------------------------------------------------------

    /// Trigger an immediate sync cycle.
    pub async fn sync_now(&self) -> Result<(), DbusError> {
        let proxy = LnxdriveSyncProxy::new(&self.connection).await?;
        Ok(proxy.sync_now().await?)
    }

    /// Pause synchronization.
    pub async fn pause(&self) -> Result<(), DbusError> {
        let proxy = LnxdriveSyncProxy::new(&self.connection).await?;
        Ok(proxy.pause().await?)
    }

    /// Resume synchronization.
    pub async fn resume(&self) -> Result<(), DbusError> {
        let proxy = LnxdriveSyncProxy::new(&self.connection).await?;
        Ok(proxy.resume().await?)
    }

    // -- Conflicts ----------------------------------------------------------

    /// List all unresolved conflicts. Returns a JSON array string.
    pub async fn list_conflicts(&self) -> Result<String, DbusError> {
        let proxy = LnxdriveConflictsProxy::new(&self.connection).await?;
        Ok(proxy.list().await?)
    }

    /// Get details for a specific conflict by ID. Returns JSON string.
    pub async fn get_conflict_details(&self, id: &str) -> Result<String, DbusError> {
        let proxy = LnxdriveConflictsProxy::new(&self.connection).await?;
        Ok(proxy.get_details(id).await?)
    }

    /// Resolve a conflict with the given strategy. Returns true on success.
    pub async fn resolve_conflict(
        &self,
        id: &str,
        strategy: &str,
    ) -> Result<bool, DbusError> {
        let proxy = LnxdriveConflictsProxy::new(&self.connection).await?;
        Ok(proxy.resolve(id, strategy).await?)
    }

    /// Resolve all unresolved conflicts with the given strategy.
    /// Returns the number of conflicts resolved.
    pub async fn resolve_all_conflicts(&self, strategy: &str) -> Result<u32, DbusError> {
        let proxy = LnxdriveConflictsProxy::new(&self.connection).await?;
        Ok(proxy.resolve_all(strategy).await?)
    }
}
