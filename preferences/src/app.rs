// LNXDrive Application — adw::Application subclass
//
// On activation the app checks the daemon's authentication state over D-Bus
// and shows the onboarding wizard or the preferences panel accordingly.

use gettextrs::gettext;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::dbus_client::DbusClient;
use crate::window::LnxdriveWindow;

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    #[derive(Default)]
    pub struct LnxdriveApp;

    #[glib::object_subclass]
    impl ObjectSubclass for LnxdriveApp {
        const NAME: &'static str = "LnxdriveApp";
        type Type = super::LnxdriveApp;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for LnxdriveApp {}

    impl ApplicationImpl for LnxdriveApp {
        fn activate(&self) {
            let app = self.obj();
            app.on_activate();
        }
    }

    impl GtkApplicationImpl for LnxdriveApp {}
    impl AdwApplicationImpl for LnxdriveApp {}
}

glib::wrapper! {
    pub struct LnxdriveApp(ObjectSubclass<imp::LnxdriveApp>)
        @extends adw::Application, gtk4::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl LnxdriveApp {
    /// Application ID following GNOME conventions.
    const APP_ID: &'static str = "com.enigmora.LNXDrive.Preferences";

    pub fn new() -> Self {
        glib::Object::builder()
            .property("application-id", Self::APP_ID)
            .property("flags", gio::ApplicationFlags::default())
            .build()
    }

    /// Called from `ApplicationImpl::activate`.
    fn on_activate(&self) {
        // Re-present existing window if already created.
        if let Some(existing) = self.active_window() {
            existing.present();
            return;
        }

        let window = LnxdriveWindow::new(self);

        // Attempt D-Bus connection and auth check asynchronously.
        let win = window.clone();
        glib::MainContext::default().spawn_local(async move {
            match DbusClient::new().await {
                Ok(client) => match client.is_authenticated().await {
                    Ok(true) => win.show_preferences(&client),
                    Ok(false) => win.show_onboarding(client),
                    Err(e) => win.show_dbus_error(&format!(
                        "{}: {}",
                        gettext("Could not query authentication state"),
                        e
                    )),
                },
                Err(e) => win.show_dbus_error(&format!(
                    "{}: {}",
                    gettext("Could not connect to LNXDrive daemon"),
                    e
                )),
            }
        });

        window.present();
    }
}

impl Default for LnxdriveApp {
    fn default() -> Self {
        Self::new()
    }
}
