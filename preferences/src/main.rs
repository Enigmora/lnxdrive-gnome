// LNXDrive Preferences & Onboarding — Entry Point
//
// Initializes gettext for i18n, creates the LnxdriveApp (adw::Application subclass),
// and runs the GTK main loop.

mod app;
mod conflicts;
mod dbus_client;
mod onboarding;
mod preferences;
mod window;

use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};
use gtk4::glib;
use gtk4::prelude::*;

use app::LnxdriveApp;

/// Locale directory configured at build time via the LOCALEDIR env var
/// (set by meson.build). Falls back to the FHS default.
const LOCALEDIR: &str = match option_env!("LOCALEDIR") {
    Some(dir) => dir,
    None => "/usr/share/locale",
};

const GETTEXT_DOMAIN: &str = "lnxdrive-gnome";

fn main() -> glib::ExitCode {
    // Initialize gettext for translatable strings.
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_DOMAIN, LOCALEDIR).expect("Failed to bind text domain");
    textdomain(GETTEXT_DOMAIN).expect("Failed to set text domain");

    let app = LnxdriveApp::new();
    app.run()
}
