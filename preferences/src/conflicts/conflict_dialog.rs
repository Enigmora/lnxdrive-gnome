// ConflictDetailDialog — adw::Dialog subclass
//
// Shows side-by-side details for a single conflict (local vs remote version)
// and lets the user choose a resolution strategy. Optionally allows creating
// a persistent rule for the file type ("Remember for this file type").

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::dbus_client::DbusClient;

// ---------------------------------------------------------------------------
// ConflictInfo — deserialized from daemon JSON
// ---------------------------------------------------------------------------

/// Lightweight struct holding the data needed to display a conflict.
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub id: String,
    pub item_id: String,
    pub item_path: String,
    pub detected_at: String,
    pub local_hash: String,
    pub local_size: u64,
    pub local_modified: String,
    pub remote_hash: String,
    pub remote_size: u64,
    pub remote_modified: String,
}

impl ConflictInfo {
    /// Parse a single conflict JSON value into a ConflictInfo.
    pub fn from_json(val: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: val.get("id")?.as_str()?.to_string(),
            item_id: val.get("item_id")?.as_str()?.to_string(),
            item_path: val
                .get("item_path")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            detected_at: val.get("detected_at")?.as_str()?.to_string(),
            local_hash: val
                .get("local_version")
                .and_then(|v| v.get("hash"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            local_size: val
                .get("local_version")
                .and_then(|v| v.get("size_bytes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            local_modified: val
                .get("local_version")
                .and_then(|v| v.get("modified_at"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            remote_hash: val
                .get("remote_version")
                .and_then(|v| v.get("hash"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            remote_size: val
                .get("remote_version")
                .and_then(|v| v.get("size_bytes"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            remote_modified: val
                .get("remote_version")
                .and_then(|v| v.get("modified_at"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    /// Parse a JSON array string into a list of ConflictInfo.
    pub fn from_json_array(json_str: &str) -> Vec<Self> {
        let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) else {
            return Vec::new();
        };
        arr.iter().filter_map(Self::from_json).collect()
    }

    /// Return the filename (last path component).
    pub fn filename(&self) -> &str {
        self.item_path
            .rsplit('/')
            .next()
            .unwrap_or(&self.item_path)
    }

    /// Return the file extension, if any.
    pub fn extension(&self) -> Option<&str> {
        self.item_path.rsplit('.').next()
    }
}

/// Format a byte count into a human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB", "TB"];
    let k = 1024_f64;
    let i = (bytes as f64).ln() / k.ln();
    let i = i.floor() as usize;
    let i = i.min(units.len() - 1);
    let value = bytes as f64 / k.powi(i as i32);
    if i == 0 {
        format!("{} {}", value as u64, units[i])
    } else {
        format!("{:.1} {}", value, units[i])
    }
}

// ---------------------------------------------------------------------------
// ConflictDetailDialog
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct ConflictDetailDialog {
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub conflict_id: RefCell<String>,
    }

    impl Default for ConflictDetailDialog {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
                conflict_id: RefCell::new(String::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConflictDetailDialog {
        const NAME: &'static str = "LnxdriveConflictDetailDialog";
        type Type = super::ConflictDetailDialog;
        type ParentType = adw::Dialog;
    }

    impl ObjectImpl for ConflictDetailDialog {}
    impl WidgetImpl for ConflictDetailDialog {}
    impl AdwDialogImpl for ConflictDetailDialog {}
}

glib::wrapper! {
    pub struct ConflictDetailDialog(ObjectSubclass<imp::ConflictDetailDialog>)
        @extends adw::Dialog, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ConflictDetailDialog {
    /// Create and populate the dialog for a given conflict.
    pub fn new(conflict: &ConflictInfo, dbus_client: &DbusClient) -> Self {
        let dialog: Self = glib::Object::builder()
            .property("title", gettext("Resolve Conflict"))
            .build();

        dialog
            .imp()
            .dbus_client
            .replace(Some(dbus_client.clone()));
        dialog
            .imp()
            .conflict_id
            .replace(conflict.id.clone());

        dialog.build_ui(conflict);
        dialog
    }

    fn build_ui(&self, conflict: &ConflictInfo) {
        let toolbar_view = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        toolbar_view.add_top_bar(&header);

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.set_margin_top(12);
        content.set_margin_bottom(24);

        // -- File info header ------------------------------------------------
        let file_label = gtk4::Label::builder()
            .label(conflict.filename())
            .css_classes(["title-2"])
            .halign(gtk4::Align::Start)
            .build();
        content.append(&file_label);

        let path_label = gtk4::Label::builder()
            .label(&conflict.item_path)
            .css_classes(["dim-label"])
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .build();
        content.append(&path_label);

        // -- Side-by-side version comparison ----------------------------------
        let comparison_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        comparison_box.set_homogeneous(true);

        // Local version
        let local_group = adw::PreferencesGroup::builder()
            .title(&gettext("Local Version"))
            .build();
        let local_size_row = adw::ActionRow::builder()
            .title(&gettext("Size"))
            .subtitle(&format_bytes(conflict.local_size))
            .build();
        let local_modified_row = adw::ActionRow::builder()
            .title(&gettext("Modified"))
            .subtitle(&conflict.local_modified)
            .build();
        let local_hash_row = adw::ActionRow::builder()
            .title(&gettext("Hash"))
            .subtitle(&conflict.local_hash)
            .build();
        local_group.add(&local_size_row);
        local_group.add(&local_modified_row);
        local_group.add(&local_hash_row);

        // Remote version
        let remote_group = adw::PreferencesGroup::builder()
            .title(&gettext("Remote Version"))
            .build();
        let remote_size_row = adw::ActionRow::builder()
            .title(&gettext("Size"))
            .subtitle(&format_bytes(conflict.remote_size))
            .build();
        let remote_modified_row = adw::ActionRow::builder()
            .title(&gettext("Modified"))
            .subtitle(&conflict.remote_modified)
            .build();
        let remote_hash_row = adw::ActionRow::builder()
            .title(&gettext("Hash"))
            .subtitle(&conflict.remote_hash)
            .build();
        remote_group.add(&remote_size_row);
        remote_group.add(&remote_modified_row);
        remote_group.add(&remote_hash_row);

        comparison_box.append(&local_group);
        comparison_box.append(&remote_group);
        content.append(&comparison_box);

        // -- Resolution actions -----------------------------------------------
        let actions_group = adw::PreferencesGroup::builder()
            .title(&gettext("Resolution"))
            .build();

        let keep_local_row = adw::ActionRow::builder()
            .title(&gettext("Keep Local"))
            .subtitle(&gettext("Upload the local version, overwriting the remote"))
            .activatable(true)
            .build();
        keep_local_row.add_suffix(&gtk4::Image::from_icon_name("go-up-symbolic"));

        let keep_remote_row = adw::ActionRow::builder()
            .title(&gettext("Keep Remote"))
            .subtitle(&gettext("Download the remote version, overwriting the local"))
            .activatable(true)
            .build();
        keep_remote_row.add_suffix(&gtk4::Image::from_icon_name("go-down-symbolic"));

        let keep_both_row = adw::ActionRow::builder()
            .title(&gettext("Keep Both"))
            .subtitle(&gettext("Rename the local file and download the remote version"))
            .activatable(true)
            .build();
        keep_both_row.add_suffix(&gtk4::Image::from_icon_name("edit-copy-symbolic"));

        actions_group.add(&keep_local_row);
        actions_group.add(&keep_remote_row);
        actions_group.add(&keep_both_row);
        content.append(&actions_group);

        // -- Connect resolution actions ---------------------------------------
        let dialog_ref = self.clone();
        keep_local_row.connect_activated(move |_| {
            dialog_ref.resolve_with_strategy("keep_local");
        });

        let dialog_ref = self.clone();
        keep_remote_row.connect_activated(move |_| {
            dialog_ref.resolve_with_strategy("keep_remote");
        });

        let dialog_ref = self.clone();
        keep_both_row.connect_activated(move |_| {
            dialog_ref.resolve_with_strategy("keep_both");
        });

        // -- Scrolled window for content --------------------------------------
        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&content)
            .build();

        toolbar_view.set_content(Some(&scrolled));

        self.set_content_width(600);
        self.set_content_height(500);
        self.set_child(Some(&toolbar_view));
    }

    fn resolve_with_strategy(&self, strategy: &str) {
        let imp = self.imp();
        let client: DbusClient = match imp.dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };
        let conflict_id = imp.conflict_id.borrow().clone();
        let strategy = strategy.to_string();
        let dialog = self.clone();

        glib::MainContext::default().spawn_local(async move {
            match client.resolve_conflict(&conflict_id, &strategy).await {
                Ok(true) => {
                    dialog.close();
                }
                Ok(false) => {
                    eprintln!("Failed to resolve conflict {conflict_id}: daemon returned false");
                }
                Err(e) => {
                    eprintln!("D-Bus error resolving conflict {conflict_id}: {e}");
                }
            }
        });
    }
}
