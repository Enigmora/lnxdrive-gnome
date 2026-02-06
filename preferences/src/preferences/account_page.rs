// Account Page — adw::PreferencesPage subclass
//
// Displays OneDrive account information (email, display name), storage quota
// with a LevelBar, and a "Sign Out" button that logs out and returns to
// onboarding.

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;

// ---------------------------------------------------------------------------
// AccountPage — adw::PreferencesPage subclass
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct AccountPage {
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub email_row: RefCell<Option<adw::ActionRow>>,
        pub name_row: RefCell<Option<adw::ActionRow>>,
        pub level_bar: RefCell<Option<gtk4::LevelBar>>,
        pub quota_label: RefCell<Option<gtk4::Label>>,
    }

    impl Default for AccountPage {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
                email_row: RefCell::new(None),
                name_row: RefCell::new(None),
                level_bar: RefCell::new(None),
                quota_label: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AccountPage {
        const NAME: &'static str = "LnxdriveAccountPage";
        type Type = super::AccountPage;
        type ParentType = adw::PreferencesPage;
    }

    impl ObjectImpl for AccountPage {}
    impl WidgetImpl for AccountPage {}
    impl PreferencesPageImpl for AccountPage {}
}

glib::wrapper! {
    pub struct AccountPage(ObjectSubclass<imp::AccountPage>)
        @extends adw::PreferencesPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl AccountPage {
    pub fn new(dbus_client: &DbusClient) -> Self {
        let page: Self = glib::Object::builder()
            .property("icon-name", "user-info-symbolic")
            .property("title", gettext("Account"))
            .build();

        page.imp()
            .dbus_client
            .replace(Some(dbus_client.clone()));

        page.build_ui();
        page.load_account_info();
        page.load_quota();

        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // -- OneDrive Account group ------------------------------------------

        let account_group = adw::PreferencesGroup::builder()
            .title(&gettext("OneDrive Account"))
            .build();

        let email_row = adw::ActionRow::builder()
            .title(&gettext("Email"))
            .subtitle(&gettext("Loading..."))
            .build();
        imp.email_row.replace(Some(email_row.clone()));

        let name_row = adw::ActionRow::builder()
            .title(&gettext("Display Name"))
            .subtitle(&gettext("Loading..."))
            .build();
        imp.name_row.replace(Some(name_row.clone()));

        account_group.add(&email_row);
        account_group.add(&name_row);

        // -- Storage group ---------------------------------------------------

        let storage_group = adw::PreferencesGroup::builder()
            .title(&gettext("Storage"))
            .build();

        let level_bar = gtk4::LevelBar::builder()
            .min_value(0.0)
            .max_value(1.0)
            .value(0.0)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(4)
            .build();
        imp.level_bar.replace(Some(level_bar.clone()));

        let quota_label = gtk4::Label::builder()
            .label(&gettext("Loading storage info..."))
            .css_classes(["dim-label", "caption"])
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .halign(gtk4::Align::Start)
            .build();
        imp.quota_label.replace(Some(quota_label.clone()));

        // Wrap the level bar and label inside a Box, then add to the group.
        let storage_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        storage_box.append(&level_bar);
        storage_box.append(&quota_label);

        // Use a ListBox row-like wrapper via a generic widget in the group.
        // PreferencesGroup expects rows but we can use a raw gtk::ListBoxRow.
        let storage_row = gtk4::ListBoxRow::builder()
            .activatable(false)
            .selectable(false)
            .child(&storage_box)
            .build();
        storage_group.add(&storage_row);

        // -- Session group ---------------------------------------------------

        let session_group = adw::PreferencesGroup::builder()
            .title(&gettext("Session"))
            .build();

        let sign_out_button = gtk4::Button::builder()
            .label(&gettext("Sign Out"))
            .halign(gtk4::Align::Center)
            .css_classes(["destructive-action", "pill"])
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let sign_out_row = gtk4::ListBoxRow::builder()
            .activatable(false)
            .selectable(false)
            .child(&sign_out_button)
            .build();
        session_group.add(&sign_out_row);

        // Connect sign-out button.
        let page = self.clone();
        sign_out_button.connect_clicked(move |_| {
            page.on_sign_out();
        });

        // Add all groups to the page.
        self.add(&account_group);
        self.add(&storage_group);
        self.add(&session_group);
    }

    /// Fetch account information from the daemon and populate the rows.
    fn load_account_info(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let page = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_account_info().await {
                Ok(info) => {
                    let email = info
                        .get("email")
                        .and_then(|v| String::try_from(v.clone()).ok())
                        .unwrap_or_else(|| gettext("Unknown"));
                    let display_name = info
                        .get("display_name")
                        .and_then(|v| String::try_from(v.clone()).ok())
                        .unwrap_or_else(|| gettext("Unknown"));

                    if let Some(ref row) = *page.imp().email_row.borrow() {
                        row.set_subtitle(&email);
                    }
                    if let Some(ref row) = *page.imp().name_row.borrow() {
                        row.set_subtitle(&display_name);
                    }
                }
                Err(e) => {
                    let error_msg = format!("{}: {}", gettext("Could not load account info"), e);
                    if let Some(ref row) = *page.imp().email_row.borrow() {
                        row.set_subtitle(&error_msg);
                    }
                }
            }
        });
    }

    /// Fetch quota information and update the level bar and label.
    fn load_quota(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let page = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_quota().await {
                Ok((used, total)) => {
                    page.update_quota_display(used, total);
                }
                Err(e) => {
                    if let Some(ref label) = *page.imp().quota_label.borrow() {
                        label.set_label(&format!(
                            "{}: {}",
                            gettext("Could not load quota"),
                            e
                        ));
                    }
                }
            }
        });
    }

    /// Update the quota level bar and label with the given byte values.
    fn update_quota_display(&self, used_bytes: u64, total_bytes: u64) {
        let imp = self.imp();

        let fraction = if total_bytes > 0 {
            used_bytes as f64 / total_bytes as f64
        } else {
            0.0
        };

        if let Some(ref bar) = *imp.level_bar.borrow() {
            bar.set_value(fraction);
        }

        let used_gb = used_bytes as f64 / 1_073_741_824.0;
        let total_gb = total_bytes as f64 / 1_073_741_824.0;

        let text = format!(
            "{:.1} GB {} {:.1} GB {}",
            used_gb,
            gettext("of"),
            total_gb,
            gettext("used")
        );

        if let Some(ref label) = *imp.quota_label.borrow() {
            label.set_label(&text);
        }
    }

    /// Prompt the user to confirm sign-out, then log out via D-Bus and switch
    /// back to the onboarding view.
    fn on_sign_out(&self) {
        // Create a confirmation dialog.
        let confirm = adw::AlertDialog::builder()
            .heading(&gettext("Sign Out?"))
            .body(&gettext(
                "You will be signed out of your OneDrive account. Syncing will stop.",
            ))
            .build();

        confirm.add_response("cancel", &gettext("Cancel"));
        confirm.add_response("sign-out", &gettext("Sign Out"));
        confirm.set_response_appearance("sign-out", adw::ResponseAppearance::Destructive);
        confirm.set_default_response(Some("cancel"));
        confirm.set_close_response("cancel");

        let page = self.clone();
        confirm.connect_response(None, move |_dialog, response| {
            if response == "sign-out" {
                page.perform_logout();
            }
        });

        // Present the alert dialog relative to this page widget.
        adw::prelude::AdwDialogExt::present(&confirm, Some(self.upcast_ref::<gtk4::Widget>()));
    }

    /// Execute the logout D-Bus call and switch to onboarding.
    fn perform_logout(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        // Find the application's active window before we lose context.
        // LnxdriveWindow doesn't implement IsA<Root>, so we go through
        // the application's active window list instead.
        let app_window: Option<crate::window::LnxdriveWindow> =
            gtk4::gio::Application::default()
                .and_then(|app| app.downcast::<gtk4::Application>().ok())
                .and_then(|app| app.active_window())
                .and_then(|win| win.downcast::<crate::window::LnxdriveWindow>().ok());

        // Close the preferences dialog if we can find it in the ancestry.
        // The PreferencesDialog is an adw::Dialog which is NOT a gtk::Window,
        // so we use force_close via the parent dialog mechanism.
        if let Some(ancestor) = self.ancestor(adw::PreferencesDialog::static_type()) {
            if let Ok(dialog) = ancestor.downcast::<adw::PreferencesDialog>() {
                dialog.force_close();
            }
        }

        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = client.logout().await {
                eprintln!("Logout error: {}", e);
            }

            // Switch the main window to onboarding.
            if let Some(window) = app_window {
                window.show_onboarding(client);
            }
        });
    }
}
