// Auth Page — first step of the onboarding wizard
//
// Shows a "Sign in to OneDrive" status page with a sign-in button.
// On click: calls StartAuth() over D-Bus, opens the auth URL in the default
// browser, switches to a waiting state with a spinner, and subscribes to the
// AuthStateChanged signal.  On success, pushes the FolderPage.

use std::cell::RefCell;

use futures_util::StreamExt;
use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::LnxdriveAuthProxy;

use super::folder_page::FolderPage;
use super::OnboardingView;

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct AuthPage {
        pub onboarding_view: RefCell<Option<OnboardingView>>,
        pub status_page: RefCell<Option<adw::StatusPage>>,
        pub sign_in_button: RefCell<Option<gtk4::Button>>,
        pub spinner: RefCell<Option<gtk4::Spinner>>,
        pub cancel_button: RefCell<Option<gtk4::Button>>,
        pub error_banner: RefCell<Option<adw::Banner>>,
        pub content_box: RefCell<Option<gtk4::Box>>,
    }

    impl Default for AuthPage {
        fn default() -> Self {
            Self {
                onboarding_view: RefCell::new(None),
                status_page: RefCell::new(None),
                sign_in_button: RefCell::new(None),
                spinner: RefCell::new(None),
                cancel_button: RefCell::new(None),
                error_banner: RefCell::new(None),
                content_box: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AuthPage {
        const NAME: &'static str = "LnxdriveAuthPage";
        type Type = super::AuthPage;
        type ParentType = adw::NavigationPage;
    }

    impl ObjectImpl for AuthPage {}
    impl WidgetImpl for AuthPage {}
    impl NavigationPageImpl for AuthPage {}
}

glib::wrapper! {
    pub struct AuthPage(ObjectSubclass<imp::AuthPage>)
        @extends adw::NavigationPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl AuthPage {
    pub fn new(onboarding_view: &OnboardingView) -> Self {
        let page: Self = glib::Object::builder()
            .property("title", gettext("Sign In"))
            .property("tag", "auth")
            .build();

        page.imp()
            .onboarding_view
            .replace(Some(onboarding_view.clone()));

        page.build_ui();
        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // Error banner (hidden by default)
        let error_banner = adw::Banner::new("");
        error_banner.set_revealed(false);
        imp.error_banner.replace(Some(error_banner.clone()));

        // Sign-in button
        let sign_in_button = gtk4::Button::builder()
            .label(&gettext("Sign In"))
            .halign(gtk4::Align::Center)
            .css_classes(["suggested-action", "pill"])
            .build();
        imp.sign_in_button
            .replace(Some(sign_in_button.clone()));

        // Waiting-state spinner (hidden initially)
        let spinner = gtk4::Spinner::builder()
            .spinning(false)
            .visible(false)
            .halign(gtk4::Align::Center)
            .build();
        imp.spinner.replace(Some(spinner.clone()));

        // Waiting-state cancel button (hidden initially)
        let cancel_button = gtk4::Button::builder()
            .label(&gettext("Cancel"))
            .halign(gtk4::Align::Center)
            .css_classes(["destructive-action", "pill"])
            .visible(false)
            .build();
        imp.cancel_button
            .replace(Some(cancel_button.clone()));

        // Waiting label (hidden initially, placed next to spinner)
        let waiting_label = gtk4::Label::builder()
            .label(&gettext("Waiting for authentication..."))
            .visible(false)
            .build();

        // Status page
        let status_page = adw::StatusPage::builder()
            .icon_name("dialog-password-symbolic")
            .title(&gettext("Sign in to OneDrive"))
            .description(&gettext(
                "Connect your Microsoft account to start syncing files.",
            ))
            .build();

        // Button box
        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .halign(gtk4::Align::Center)
            .build();
        button_box.append(&sign_in_button);
        button_box.append(&spinner);
        button_box.append(&waiting_label.clone());
        button_box.append(&cancel_button);

        status_page.set_child(Some(&button_box));
        imp.status_page.replace(Some(status_page.clone()));

        // Outer layout: banner on top, status page below
        let content_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.add_top_bar(&error_banner);
        toolbar_view.set_content(Some(&status_page));

        content_box.append(&toolbar_view);
        imp.content_box.replace(Some(content_box.clone()));

        self.set_child(Some(&content_box));

        // Connect sign-in button click
        let page = self.clone();
        let waiting_label_clone = waiting_label.clone();
        sign_in_button.connect_clicked(move |_| {
            page.on_sign_in_clicked(&waiting_label_clone);
        });

        // Connect cancel button
        let page = self.clone();
        let waiting_label_clone2 = waiting_label;
        cancel_button.connect_clicked(move |_| {
            page.on_cancel_clicked(&waiting_label_clone2);
        });
    }

    /// Called when the user clicks "Sign In".
    fn on_sign_in_clicked(&self, waiting_label: &gtk4::Label) {
        let imp = self.imp();

        let onboarding_view = match imp.onboarding_view.borrow().clone() {
            Some(v) => v,
            None => return,
        };

        let dbus_client = match onboarding_view.dbus_client().as_ref() {
            Some(c) => c.clone(),
            None => return,
        };

        // Switch to waiting state
        self.set_waiting_state(true, waiting_label);

        let page = self.clone();
        let ov = onboarding_view.clone();
        let wl = waiting_label.clone();

        glib::MainContext::default().spawn_local(async move {
            // 1. Call StartAuth() to get the browser URL
            match dbus_client.start_auth().await {
                Ok((auth_url, _state)) => {
                    // 2. Open the URL in the default browser
                    let launcher = gtk4::UriLauncher::new(&auth_url);

                    if let Some(win) = ov.parent_window() {
                        if let Err(e) = launcher.launch_future(Some(&win)).await {
                            page.show_error(&format!(
                                "{}: {}",
                                gettext("Could not open browser"),
                                e
                            ));
                            page.set_waiting_state(false, &wl);
                            return;
                        }
                    }

                    // 3. Subscribe to AuthStateChanged signal.
                    // Clone the connection so the proxy doesn't borrow dbus_client,
                    // allowing us to call other methods on dbus_client while the
                    // signal stream is active.
                    let conn = dbus_client.connection().clone();
                    match LnxdriveAuthProxy::new(&conn).await {
                        Ok(proxy) => match proxy.receive_auth_state_changed().await {
                            Ok(mut stream) => {
                                while let Some(signal) = stream.next().await {
                                    if let Ok(args) = signal.args() {
                                        match args.state {
                                            "authenticated" => {
                                                // Fetch account info for state
                                                if let Ok(info) =
                                                    dbus_client.get_account_info().await
                                                {
                                                    let mut ob_state = ov.state_mut();
                                                    ob_state.account_email = info
                                                        .get("email")
                                                        .and_then(|v| {
                                                            String::try_from(v.clone()).ok()
                                                        });
                                                    ob_state.account_name = info
                                                        .get("display_name")
                                                        .and_then(|v| {
                                                            String::try_from(v.clone()).ok()
                                                        });
                                                }

                                                // Push the folder selection page
                                                let folder_page = FolderPage::new(&ov);
                                                ov.nav_view().push(&folder_page);
                                                page.set_waiting_state(false, &wl);
                                                return;
                                            }
                                            "error" => {
                                                page.show_error(&gettext(
                                                    "Authentication failed. Please try again.",
                                                ));
                                                page.set_waiting_state(false, &wl);
                                                return;
                                            }
                                            _ => {
                                                // Other transient states; keep waiting.
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                page.show_error(&format!(
                                    "{}: {}",
                                    gettext("Could not listen for auth events"),
                                    e
                                ));
                                page.set_waiting_state(false, &wl);
                            }
                        },
                        Err(e) => {
                            page.show_error(&format!(
                                "{}: {}",
                                gettext("D-Bus proxy error"),
                                e
                            ));
                            page.set_waiting_state(false, &wl);
                        }
                    }
                }
                Err(e) => {
                    page.show_error(&format!(
                        "{}: {}",
                        gettext("Could not start authentication"),
                        e
                    ));
                    page.set_waiting_state(false, &wl);
                }
            }
        });
    }

    /// Toggle between the initial "Sign In" state and the waiting/spinner state.
    fn set_waiting_state(&self, waiting: bool, waiting_label: &gtk4::Label) {
        let imp = self.imp();

        if let Some(ref btn) = *imp.sign_in_button.borrow() {
            btn.set_visible(!waiting);
        }
        if let Some(ref spinner) = *imp.spinner.borrow() {
            spinner.set_visible(waiting);
            spinner.set_spinning(waiting);
        }
        waiting_label.set_visible(waiting);
        if let Some(ref cancel) = *imp.cancel_button.borrow() {
            cancel.set_visible(waiting);
        }
    }

    /// Cancel the ongoing authentication attempt and reset the wizard.
    fn on_cancel_clicked(&self, waiting_label: &gtk4::Label) {
        self.set_waiting_state(false, waiting_label);

        if let Some(ref ov) = *self.imp().onboarding_view.borrow() {
            ov.on_cancel();
        }
    }

    /// Display an inline error banner at the top of the page.
    fn show_error(&self, message: &str) {
        if let Some(ref banner) = *self.imp().error_banner.borrow() {
            banner.set_title(message);
            banner.set_revealed(true);
        }
    }
}
