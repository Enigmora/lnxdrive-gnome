// LNXDrive Main Window — adw::ApplicationWindow subclass
//
// Hosts either the onboarding wizard (NavigationView) or the preferences panel.
// Persists window geometry via GSettings.

use gettextrs::gettext;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::dbus_client::DbusClient;
use crate::onboarding::OnboardingView;
use crate::preferences::PreferencesDialog;

mod imp {
    use super::*;
    use std::cell::RefCell;

    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    #[derive(Default)]
    pub struct LnxdriveWindow {
        pub settings: RefCell<Option<gio::Settings>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for LnxdriveWindow {
        const NAME: &'static str = "LnxdriveWindow";
        type Type = super::LnxdriveWindow;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for LnxdriveWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            // Load GSettings for window geometry persistence.
            let settings = gio::Settings::new("com.enigmora.LNXDrive.Preferences");

            let width = settings.int("window-width");
            let height = settings.int("window-height");
            obj.set_default_size(width, height);

            *self.settings.borrow_mut() = Some(settings);

            obj.set_title(Some(&gettext("LNXDrive")));
        }
    }

    impl WidgetImpl for LnxdriveWindow {}

    impl WindowImpl for LnxdriveWindow {
        fn close_request(&self) -> glib::Propagation {
            // Persist the current window size to GSettings.
            if let Some(ref settings) = *self.settings.borrow() {
                let obj = self.obj();
                let (width, height) = obj.default_size();
                let _ = settings.set_int("window-width", width);
                let _ = settings.set_int("window-height", height);
            }

            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for LnxdriveWindow {}
    impl AdwApplicationWindowImpl for LnxdriveWindow {}
}

glib::wrapper! {
    pub struct LnxdriveWindow(ObjectSubclass<imp::LnxdriveWindow>)
        @extends adw::ApplicationWindow, gtk4::ApplicationWindow,
                 gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl LnxdriveWindow {
    pub fn new(app: &crate::app::LnxdriveApp) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    /// Replace the window content with the onboarding wizard.
    pub fn show_onboarding(&self, dbus_client: DbusClient) {
        let onboarding = OnboardingView::new(dbus_client, self.clone());
        self.set_content(Some(&onboarding));
    }

    /// Set the window content to a "connected" status page and present the
    /// preferences dialog on top. The underlying window content acts as the
    /// backdrop while the PreferencesDialog is open.
    pub fn show_preferences(&self, dbus_client: &DbusClient) {
        // Set up window content behind the dialog.
        let status = adw::StatusPage::builder()
            .icon_name("emblem-ok-symbolic")
            .title(&gettext("LNXDrive"))
            .description(&gettext("Your OneDrive files are syncing."))
            .build();

        // Add a button to re-open preferences if the dialog is closed.
        let open_prefs_button = gtk4::Button::builder()
            .label(&gettext("Preferences"))
            .halign(gtk4::Align::Center)
            .css_classes(["pill"])
            .build();
        status.set_child(Some(&open_prefs_button));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.set_content(Some(&status));

        self.set_content(Some(&toolbar_view));

        // Connect the button to re-open preferences.
        let client = dbus_client.clone();
        let win = self.clone();
        open_prefs_button.connect_clicked(move |_| {
            let dialog = PreferencesDialog::new(&client);
            dialog.present(&win);
        });

        // Present the dialog immediately.
        let dialog = PreferencesDialog::new(dbus_client);
        dialog.present(self);
    }

    /// Show an error status page when the D-Bus daemon is unreachable.
    pub fn show_dbus_error(&self, message: &str) {
        let status = adw::StatusPage::builder()
            .icon_name("dialog-error-symbolic")
            .title(&gettext("Cannot Connect to LNXDrive"))
            .description(message)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.set_content(Some(&status));

        self.set_content(Some(&toolbar_view));
    }
}
