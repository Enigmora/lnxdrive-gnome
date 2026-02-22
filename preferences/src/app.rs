// LNXDrive Application — adw::Application subclass
//
// On activation the app checks the daemon's authentication state over D-Bus
// and shows the onboarding wizard or the preferences panel accordingly.

use gettextrs::gettext;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use libadwaita as adw;
use libadwaita::prelude::*;

use crate::dbus_client::DbusClient;
use crate::window::LnxdriveWindow;

mod imp {
    use super::*;
    use std::cell::OnceCell;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    #[derive(Default)]
    pub struct LnxdriveApp {
        pub initial_page: OnceCell<Option<String>>,
    }

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

        fn command_line(&self, command_line: &gio::ApplicationCommandLine) -> glib::ExitCode {
            let page = command_line
                .options_dict()
                .lookup::<String>("page")
                .ok()
                .flatten();
            let _ = self.initial_page.set(page);
            self.obj().activate();
            glib::ExitCode::SUCCESS
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
        let app: Self = glib::Object::builder()
            .property("application-id", Self::APP_ID)
            .property("flags", gio::ApplicationFlags::HANDLES_COMMAND_LINE)
            .build();

        // Register --page as a known option so GApplication doesn't reject it.
        app.add_main_option(
            "page",
            glib::Char(0),
            glib::OptionFlags::NONE,
            glib::OptionArg::String,
            "Navigate directly to a preferences page",
            Some("PAGE"),
        );

        app
    }

    /// Called from `ApplicationImpl::activate`.
    fn on_activate(&self) {
        // Re-present existing window if already created.
        if let Some(existing) = self.active_window() {
            existing.present();
            return;
        }

        let initial_page = self
            .imp()
            .initial_page
            .get()
            .and_then(|p: &Option<String>| p.clone());

        let window = LnxdriveWindow::new(self);

        // Attempt D-Bus connection and auth check asynchronously.
        let win = window.clone();
        glib::MainContext::default().spawn_local(async move {
            match DbusClient::new().await {
                Ok(client) => match client.is_authenticated().await {
                    Ok(true) => win.show_preferences(
                        &client,
                        initial_page.as_deref(),
                    ),
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
