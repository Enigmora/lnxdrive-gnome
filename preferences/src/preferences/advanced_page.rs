// Advanced Page — adw::PreferencesPage subclass
//
// Contains exclusion patterns (FR-015) and bandwidth limit controls (FR-017).
// Patterns are displayed in a ListBox with per-row delete buttons and a text
// entry for adding new patterns. Bandwidth limits use adw::SpinRow widgets.

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;

// ---------------------------------------------------------------------------
// AdvancedPage — adw::PreferencesPage subclass
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct AdvancedPage {
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub patterns_list: RefCell<Option<gtk4::ListBox>>,
        pub patterns_store: RefCell<Vec<String>>,
        pub pattern_entry: RefCell<Option<gtk4::Entry>>,
        pub upload_row: RefCell<Option<adw::SpinRow>>,
        pub download_row: RefCell<Option<adw::SpinRow>>,
        pub debounce_source: RefCell<Option<glib::SourceId>>,
    }

    impl Default for AdvancedPage {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
                patterns_list: RefCell::new(None),
                patterns_store: RefCell::new(Vec::new()),
                pattern_entry: RefCell::new(None),
                upload_row: RefCell::new(None),
                download_row: RefCell::new(None),
                debounce_source: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AdvancedPage {
        const NAME: &'static str = "LnxdriveAdvancedPage";
        type Type = super::AdvancedPage;
        type ParentType = adw::PreferencesPage;
    }

    impl ObjectImpl for AdvancedPage {}
    impl WidgetImpl for AdvancedPage {}
    impl PreferencesPageImpl for AdvancedPage {}
}

glib::wrapper! {
    pub struct AdvancedPage(ObjectSubclass<imp::AdvancedPage>)
        @extends adw::PreferencesPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl AdvancedPage {
    pub fn new(dbus_client: &DbusClient) -> Self {
        let page: Self = glib::Object::builder()
            .property("icon-name", "preferences-other-symbolic")
            .property("title", gettext("Advanced"))
            .build();

        page.imp()
            .dbus_client
            .replace(Some(dbus_client.clone()));

        page.build_ui();
        page.load_exclusion_patterns();
        page.load_bandwidth_limits();

        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // -- Exclusion Patterns group (FR-015) --------------------------------

        let patterns_group = adw::PreferencesGroup::builder()
            .title(&gettext("Exclusion Patterns"))
            .description(&gettext(
                "Files and folders matching these glob patterns will not be synced.",
            ))
            .build();

        let patterns_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        imp.patterns_list.replace(Some(patterns_list.clone()));

        // Wrap the list in a ListBoxRow for the preferences group.
        let list_row = gtk4::ListBoxRow::builder()
            .activatable(false)
            .selectable(false)
            .child(&patterns_list)
            .build();
        patterns_group.add(&list_row);

        // Entry + Add button for new patterns.
        let add_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(8)
            .build();

        let entry = gtk4::Entry::builder()
            .placeholder_text(&gettext("e.g. *.tmp, .git/, ~$*"))
            .hexpand(true)
            .build();
        imp.pattern_entry.replace(Some(entry.clone()));

        let add_button = gtk4::Button::builder()
            .label(&gettext("Add"))
            .css_classes(["suggested-action"])
            .build();

        add_box.append(&entry);
        add_box.append(&add_button);

        let add_row = gtk4::ListBoxRow::builder()
            .activatable(false)
            .selectable(false)
            .child(&add_box)
            .build();
        patterns_group.add(&add_row);

        // Connect "Add" button.
        let page = self.clone();
        add_button.connect_clicked(move |_| {
            page.on_add_pattern();
        });

        // Also allow adding via Enter key in the entry.
        let page = self.clone();
        entry.connect_activate(move |_| {
            page.on_add_pattern();
        });

        // -- Bandwidth Limits group (FR-017) ----------------------------------

        let bandwidth_group = adw::PreferencesGroup::builder()
            .title(&gettext("Bandwidth Limits"))
            .description(&gettext(
                "Limit upload and download speeds. Set to 0 for unlimited.",
            ))
            .build();

        let upload_row = adw::SpinRow::with_range(0.0, 100_000.0, 100.0);
        upload_row.set_title(&gettext("Upload Limit (KB/s)"));
        upload_row.set_subtitle(&gettext("0 = unlimited"));
        upload_row.set_value(0.0);
        upload_row.set_snap_to_ticks(true);
        imp.upload_row.replace(Some(upload_row.clone()));

        let download_row = adw::SpinRow::with_range(0.0, 100_000.0, 100.0);
        download_row.set_title(&gettext("Download Limit (KB/s)"));
        download_row.set_subtitle(&gettext("0 = unlimited"));
        download_row.set_value(0.0);
        download_row.set_snap_to_ticks(true);
        imp.download_row.replace(Some(download_row.clone()));

        bandwidth_group.add(&upload_row);
        bandwidth_group.add(&download_row);

        // Add groups to page.
        self.add(&patterns_group);
        self.add(&bandwidth_group);

        // Debounced save for bandwidth changes.
        let page = self.clone();
        upload_row.connect_value_notify(move |_| {
            page.schedule_bandwidth_save();
        });

        let page = self.clone();
        download_row.connect_value_notify(move |_| {
            page.schedule_bandwidth_save();
        });
    }

    // -- Exclusion Patterns --------------------------------------------------

    /// Load current exclusion patterns from the daemon.
    fn load_exclusion_patterns(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let page = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_exclusion_patterns().await {
                Ok(patterns) => {
                    *page.imp().patterns_store.borrow_mut() = patterns;
                    page.rebuild_patterns_list();
                }
                Err(e) => {
                    eprintln!("Could not load exclusion patterns: {}", e);
                }
            }
        });
    }

    /// Rebuild the ListBox rows from the current patterns_store.
    fn rebuild_patterns_list(&self) {
        let imp = self.imp();

        let list_box = match imp.patterns_list.borrow().clone() {
            Some(lb) => lb,
            None => return,
        };

        // Remove all existing rows.
        while let Some(child) = list_box.first_child() {
            list_box.remove(&child);
        }

        let patterns = imp.patterns_store.borrow().clone();
        for (index, pattern) in patterns.iter().enumerate() {
            let row = self.create_pattern_row(pattern, index);
            list_box.append(&row);
        }
    }

    /// Create a single row for an exclusion pattern with a delete button.
    fn create_pattern_row(&self, pattern: &str, _index: usize) -> adw::ActionRow {
        let row = adw::ActionRow::builder()
            .title(pattern)
            .build();

        let delete_button = gtk4::Button::builder()
            .icon_name("edit-delete-symbolic")
            .tooltip_text(&gettext("Remove pattern"))
            .valign(gtk4::Align::Center)
            .css_classes(["flat", "circular"])
            .build();

        row.add_suffix(&delete_button);

        let page = self.clone();
        let pattern_owned = pattern.to_string();
        delete_button.connect_clicked(move |_| {
            page.on_remove_pattern(&pattern_owned);
        });

        row
    }

    /// Add a new pattern from the entry field.
    fn on_add_pattern(&self) {
        let imp = self.imp();

        let pattern = match imp.pattern_entry.borrow().as_ref() {
            Some(entry) => {
                let text = entry.text().trim().to_string();
                entry.set_text("");
                text
            }
            None => return,
        };

        if pattern.is_empty() {
            return;
        }

        // Avoid duplicates.
        {
            let store = imp.patterns_store.borrow();
            if store.contains(&pattern) {
                return;
            }
        }

        imp.patterns_store.borrow_mut().push(pattern);
        self.rebuild_patterns_list();
        self.save_exclusion_patterns();
    }

    /// Remove a pattern by value.
    fn on_remove_pattern(&self, pattern: &str) {
        let imp = self.imp();

        imp.patterns_store
            .borrow_mut()
            .retain(|p| p != pattern);

        self.rebuild_patterns_list();
        self.save_exclusion_patterns();
    }

    /// Send the current patterns to the daemon.
    fn save_exclusion_patterns(&self) {
        let imp = self.imp();
        let patterns = imp.patterns_store.borrow().clone();

        let client = match imp.dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = client.set_exclusion_patterns(&patterns).await {
                eprintln!("Could not save exclusion patterns: {}", e);
            }
        });
    }

    // -- Bandwidth Limits ----------------------------------------------------

    /// Load bandwidth limits from daemon config.
    fn load_bandwidth_limits(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let page = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_config().await {
                Ok(yaml) => {
                    page.apply_bandwidth_config(&yaml);
                }
                Err(e) => {
                    eprintln!("Could not load bandwidth config: {}", e);
                }
            }
        });
    }

    /// Parse bandwidth settings from YAML and apply to spin rows.
    fn apply_bandwidth_config(&self, yaml: &str) {
        let imp = self.imp();

        for line in yaml.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "upload_limit_kbps" | "upload_limit" => {
                        if let Ok(val) = value.parse::<f64>() {
                            if let Some(ref row) = *imp.upload_row.borrow() {
                                row.set_value(val.clamp(0.0, 100_000.0));
                            }
                        }
                    }
                    "download_limit_kbps" | "download_limit" => {
                        if let Ok(val) = value.parse::<f64>() {
                            if let Some(ref row) = *imp.download_row.borrow() {
                                row.set_value(val.clamp(0.0, 100_000.0));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Schedule a debounced bandwidth save (500ms).
    fn schedule_bandwidth_save(&self) {
        let imp = self.imp();

        if let Some(source_id) = imp.debounce_source.borrow_mut().take() {
            source_id.remove();
        }

        let page = self.clone();
        let source_id = glib::timeout_add_local_once(
            std::time::Duration::from_millis(500),
            move || {
                page.save_bandwidth_limits();
            },
        );

        imp.debounce_source.replace(Some(source_id));
    }

    /// Send bandwidth limits to the daemon.
    fn save_bandwidth_limits(&self) {
        let imp = self.imp();

        let upload = imp
            .upload_row
            .borrow()
            .as_ref()
            .map(|r| r.value() as u32)
            .unwrap_or(0);

        let download = imp
            .download_row
            .borrow()
            .as_ref()
            .map(|r| r.value() as u32)
            .unwrap_or(0);

        let yaml = format!(
            "upload_limit_kbps: {}\ndownload_limit_kbps: {}\n",
            upload, download
        );

        let client = match imp.dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = client.set_config(&yaml).await {
                eprintln!("Could not save bandwidth config: {}", e);
            }
        });
    }
}
