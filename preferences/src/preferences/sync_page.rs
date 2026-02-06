// Sync Page — adw::PreferencesPage subclass
//
// Contains sync options (auto sync, conflict resolution, interval) and the
// selective sync folder tree (FolderTree widget). Loads initial values from
// the daemon and debounces changes before sending them back.

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;

use super::folder_tree::FolderTree;

// ---------------------------------------------------------------------------
// SyncPage — adw::PreferencesPage subclass
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct SyncPage {
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub auto_sync_row: RefCell<Option<adw::SwitchRow>>,
        pub conflict_row: RefCell<Option<adw::ComboRow>>,
        pub interval_row: RefCell<Option<adw::SpinRow>>,
        pub folder_tree: RefCell<Option<FolderTree>>,
        /// Source ID for the debounce timer. When a setting changes, we start a
        /// 500ms timeout; if another change arrives before it fires we reset it.
        pub debounce_source: RefCell<Option<glib::SourceId>>,
    }

    impl Default for SyncPage {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
                auto_sync_row: RefCell::new(None),
                conflict_row: RefCell::new(None),
                interval_row: RefCell::new(None),
                folder_tree: RefCell::new(None),
                debounce_source: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SyncPage {
        const NAME: &'static str = "LnxdriveSyncPage";
        type Type = super::SyncPage;
        type ParentType = adw::PreferencesPage;
    }

    impl ObjectImpl for SyncPage {}
    impl WidgetImpl for SyncPage {}
    impl PreferencesPageImpl for SyncPage {}
}

glib::wrapper! {
    pub struct SyncPage(ObjectSubclass<imp::SyncPage>)
        @extends adw::PreferencesPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

/// Conflict resolution strategy labels — order must match the index used in
/// the YAML configuration.
const CONFLICT_LABELS: &[&str] = &[
    "Always Ask",
    "Keep Local",
    "Keep Remote",
    "Keep Both",
];

impl SyncPage {
    pub fn new(dbus_client: &DbusClient) -> Self {
        let page: Self = glib::Object::builder()
            .property("icon-name", "folder-symbolic")
            .property("title", gettext("Sync"))
            .build();

        page.imp()
            .dbus_client
            .replace(Some(dbus_client.clone()));

        page.build_ui();
        page.load_initial_values();

        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // -- Sync Options group ----------------------------------------------

        let options_group = adw::PreferencesGroup::builder()
            .title(&gettext("Sync Options"))
            .build();

        // Automatic Sync switch (FR-018)
        let auto_sync_row = adw::SwitchRow::builder()
            .title(&gettext("Automatic Sync"))
            .subtitle(&gettext("Sync files automatically when changes are detected"))
            .build();
        imp.auto_sync_row.replace(Some(auto_sync_row.clone()));

        // Conflict Resolution combo (FR-016)
        let conflict_model = gtk4::StringList::new(
            &CONFLICT_LABELS
                .iter()
                .map(|s| gettext(*s))
                .collect::<Vec<_>>()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>(),
        );

        let conflict_row = adw::ComboRow::builder()
            .title(&gettext("Conflict Resolution"))
            .subtitle(&gettext("How to handle file conflicts between local and remote"))
            .model(&conflict_model)
            .build();
        imp.conflict_row.replace(Some(conflict_row.clone()));

        // Sync Interval spin row
        let interval_row = adw::SpinRow::with_range(1.0, 60.0, 1.0);
        interval_row.set_title(&gettext("Sync Interval (minutes)"));
        interval_row.set_subtitle(&gettext("How often to check for remote changes"));
        interval_row.set_value(5.0);
        interval_row.set_snap_to_ticks(true);
        imp.interval_row.replace(Some(interval_row.clone()));

        options_group.add(&auto_sync_row);
        options_group.add(&conflict_row);
        options_group.add(&interval_row);

        // -- Selective Sync group (FR-014) ------------------------------------

        let selective_group = adw::PreferencesGroup::builder()
            .title(&gettext("Selective Sync"))
            .description(&gettext(
                "Choose which remote folders to sync to this computer.",
            ))
            .build();

        let client = imp.dbus_client.borrow().clone();
        let folder_tree = FolderTree::new(client.as_ref());
        imp.folder_tree.replace(Some(folder_tree.clone()));

        // Wrap in a ListBoxRow so it fits inside a PreferencesGroup.
        let tree_row = gtk4::ListBoxRow::builder()
            .activatable(false)
            .selectable(false)
            .child(&folder_tree)
            .build();
        selective_group.add(&tree_row);

        // Add groups to page.
        self.add(&options_group);
        self.add(&selective_group);

        // Connect change signals with debounce.
        let page = self.clone();
        auto_sync_row.connect_active_notify(move |_| {
            page.schedule_save();
        });

        let page = self.clone();
        conflict_row.connect_selected_notify(move |_| {
            page.schedule_save();
        });

        let page = self.clone();
        interval_row.connect_value_notify(move |_| {
            page.schedule_save();
        });
    }

    /// Load initial setting values from the daemon.
    fn load_initial_values(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let page = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_config().await {
                Ok(yaml) => {
                    page.apply_config_yaml(&yaml);
                }
                Err(e) => {
                    eprintln!("Could not load config: {}", e);
                }
            }
        });
    }

    /// Parse the daemon's YAML config and apply values to the UI widgets.
    /// We do simple line-based parsing to avoid pulling in a full YAML crate
    /// beyond serde (the config is flat key-value).
    fn apply_config_yaml(&self, yaml: &str) {
        let imp = self.imp();

        for line in yaml.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "sync_mode" | "auto_sync" => {
                        let active = value == "true" || value == "auto" || value == "automatic";
                        if let Some(ref row) = *imp.auto_sync_row.borrow() {
                            row.set_active(active);
                        }
                    }
                    "conflict_resolution" => {
                        let idx = match value {
                            "ask" | "always_ask" => 0,
                            "keep_local" | "local" => 1,
                            "keep_remote" | "remote" => 2,
                            "keep_both" | "both" => 3,
                            _ => 0,
                        };
                        if let Some(ref row) = *imp.conflict_row.borrow() {
                            row.set_selected(idx);
                        }
                    }
                    "sync_interval" | "sync_interval_minutes" => {
                        if let Ok(mins) = value.parse::<f64>() {
                            if let Some(ref row) = *imp.interval_row.borrow() {
                                row.set_value(mins.clamp(1.0, 60.0));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Schedule a debounced save: cancel any pending timer and start a new
    /// 500ms timeout.
    fn schedule_save(&self) {
        let imp = self.imp();

        // Cancel existing timer.
        if let Some(source_id) = imp.debounce_source.borrow_mut().take() {
            source_id.remove();
        }

        let page = self.clone();
        let source_id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(500),
            move || {
                page.save_settings();
            },
        );

        imp.debounce_source.replace(Some(source_id));
    }

    /// Collect current widget values and send them to the daemon.
    fn save_settings(&self) {
        let imp = self.imp();

        let auto_sync = imp
            .auto_sync_row
            .borrow()
            .as_ref()
            .map(|r| r.is_active())
            .unwrap_or(false);

        let conflict_idx = imp
            .conflict_row
            .borrow()
            .as_ref()
            .map(|r| r.selected())
            .unwrap_or(0);

        let conflict_value = match conflict_idx {
            0 => "always_ask",
            1 => "keep_local",
            2 => "keep_remote",
            3 => "keep_both",
            _ => "always_ask",
        };

        let interval = imp
            .interval_row
            .borrow()
            .as_ref()
            .map(|r| r.value() as u32)
            .unwrap_or(5);

        let sync_mode = if auto_sync { "automatic" } else { "manual" };

        let yaml = format!(
            "sync_mode: \"{}\"\nconflict_resolution: \"{}\"\nsync_interval_minutes: {}\n",
            sync_mode, conflict_value, interval
        );

        let client = match imp.dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = client.set_config(&yaml).await {
                eprintln!("Could not save config: {}", e);
            }
        });
    }
}
