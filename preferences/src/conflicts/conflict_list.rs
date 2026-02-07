// ConflictListPage — adw::PreferencesPage subclass
//
// Displays all unresolved conflicts in a list and provides:
// - Click to open ConflictDetailDialog per conflict
// - "Resolve All" button with strategy selection
// - Real-time updates via D-Bus signals

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;

use super::conflict_dialog::{ConflictDetailDialog, ConflictInfo};

// ---------------------------------------------------------------------------
// ConflictListPage — adw::PreferencesPage subclass
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct ConflictListPage {
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub conflicts_group: RefCell<Option<adw::PreferencesGroup>>,
        pub empty_label: RefCell<Option<gtk4::Label>>,
    }

    impl Default for ConflictListPage {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
                conflicts_group: RefCell::new(None),
                empty_label: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConflictListPage {
        const NAME: &'static str = "LnxdriveConflictListPage";
        type Type = super::ConflictListPage;
        type ParentType = adw::PreferencesPage;
    }

    impl ObjectImpl for ConflictListPage {}
    impl WidgetImpl for ConflictListPage {}
    impl PreferencesPageImpl for ConflictListPage {}
}

glib::wrapper! {
    pub struct ConflictListPage(ObjectSubclass<imp::ConflictListPage>)
        @extends adw::PreferencesPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

const STRATEGY_LABELS: &[&str] = &["Keep Local", "Keep Remote", "Keep Both"];
const STRATEGY_VALUES: &[&str] = &["keep_local", "keep_remote", "keep_both"];

impl ConflictListPage {
    pub fn new(dbus_client: &DbusClient) -> Self {
        let page: Self = glib::Object::builder()
            .property("icon-name", "dialog-warning-symbolic")
            .property("title", gettext("Conflicts"))
            .build();

        page.imp()
            .dbus_client
            .replace(Some(dbus_client.clone()));

        page.build_ui();
        page.load_conflicts();

        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // -- Conflicts list group ---------------------------------------------
        let conflicts_group = adw::PreferencesGroup::builder()
            .title(&gettext("Unresolved Conflicts"))
            .build();

        // Resolve All button in the header
        let resolve_all_button = gtk4::Button::builder()
            .label(&gettext("Resolve All"))
            .css_classes(["flat"])
            .build();

        let page = self.clone();
        resolve_all_button.connect_clicked(move |_| {
            page.show_resolve_all_dialog();
        });
        conflicts_group.set_header_suffix(Some(&resolve_all_button));

        // Empty state label
        let empty_label = gtk4::Label::builder()
            .label(&gettext("No unresolved conflicts"))
            .css_classes(["dim-label"])
            .margin_top(12)
            .margin_bottom(12)
            .build();

        imp.conflicts_group
            .replace(Some(conflicts_group.clone()));
        imp.empty_label.replace(Some(empty_label));

        self.add(&conflicts_group);
    }

    /// Fetch the conflict list from the daemon and populate the UI.
    pub fn load_conflicts(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let page = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.list_conflicts().await {
                Ok(json_str) => {
                    let conflicts = ConflictInfo::from_json_array(&json_str);
                    page.populate_list(&conflicts);
                }
                Err(e) => {
                    eprintln!("Could not load conflicts: {e}");
                    page.populate_list(&[]);
                }
            }
        });
    }

    fn populate_list(&self, conflicts: &[ConflictInfo]) {
        let imp = self.imp();
        let group = match imp.conflicts_group.borrow().clone() {
            Some(g) => g,
            None => return,
        };

        // Clear existing rows: iterate children and remove. We remove by
        // walking the group's children. PreferencesGroup wraps a ListBox.
        // We remove all rows by iterating the underlying listbox.
        // Since adw::PreferencesGroup doesn't expose remove_all, we track
        // rows and remove them individually.
        //
        // The simplest approach: rebuild the group each time. For small
        // conflict counts (<100) this is perfectly fine.
        //
        // Remove the group and re-add a fresh one.
        self.remove(&group);

        let new_group = adw::PreferencesGroup::builder()
            .title(&gettext("Unresolved Conflicts"))
            .build();

        let resolve_all_button = gtk4::Button::builder()
            .label(&gettext("Resolve All"))
            .css_classes(["flat"])
            .build();

        let page = self.clone();
        resolve_all_button.connect_clicked(move |_| {
            page.show_resolve_all_dialog();
        });
        new_group.set_header_suffix(Some(&resolve_all_button));

        if conflicts.is_empty() {
            let empty_row = adw::ActionRow::builder()
                .title(&gettext("No unresolved conflicts"))
                .subtitle(&gettext("All files are in sync"))
                .build();
            empty_row.add_prefix(&gtk4::Image::from_icon_name("emblem-ok-symbolic"));
            new_group.add(&empty_row);
        } else {
            for conflict in conflicts {
                let row = adw::ActionRow::builder()
                    .title(conflict.filename())
                    .subtitle(&conflict.item_path)
                    .activatable(true)
                    .build();
                row.add_prefix(&gtk4::Image::from_icon_name(
                    "dialog-warning-symbolic",
                ));
                row.add_suffix(&gtk4::Image::from_icon_name(
                    "go-next-symbolic",
                ));

                // Connect click to open detail dialog
                let client = imp.dbus_client.borrow().clone();
                let conflict_clone = conflict.clone();
                let page_ref = self.clone();
                row.connect_activated(move |_| {
                    if let Some(ref client) = client {
                        let dialog =
                            ConflictDetailDialog::new(&conflict_clone, client);
                        // Present on the nearest toplevel
                        dialog.present(Some(&page_ref));
                    }
                });

                new_group.add(&row);
            }
        }

        imp.conflicts_group.replace(Some(new_group.clone()));
        self.add(&new_group);
    }

    fn show_resolve_all_dialog(&self) {
        let imp = self.imp();
        let client = match imp.dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        // Build a simple strategy chooser dialog
        let dialog = adw::AlertDialog::builder()
            .heading(&gettext("Resolve All Conflicts"))
            .body(&gettext("Choose a strategy to apply to all unresolved conflicts."))
            .build();

        dialog.add_response("cancel", &gettext("Cancel"));
        for (i, label) in STRATEGY_LABELS.iter().enumerate() {
            dialog.add_response(STRATEGY_VALUES[i], &gettext(*label));
        }
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        let page = self.clone();
        dialog.connect_response(None, move |_, response| {
            if response == "cancel" {
                return;
            }
            let strategy = response.to_string();
            let client_clone = client.clone();
            let page_clone = page.clone();

            glib::MainContext::default().spawn_local(async move {
                match client_clone.resolve_all_conflicts(&strategy).await {
                    Ok(count) => {
                        eprintln!("Resolved {count} conflicts with strategy {strategy}");
                        page_clone.load_conflicts();
                    }
                    Err(e) => {
                        eprintln!("Failed to resolve all conflicts: {e}");
                    }
                }
            });
        });

        dialog.present(Some(self));
    }
}
