// Confirm Page — final step of the onboarding wizard
//
// Shows a summary (account email, sync folder) and a "Start Syncing" button.
// On click: writes configuration to the daemon and triggers the first sync.

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use super::OnboardingView;

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct ConfirmPage {
        pub onboarding_view: RefCell<Option<OnboardingView>>,
    }

    impl Default for ConfirmPage {
        fn default() -> Self {
            Self {
                onboarding_view: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConfirmPage {
        const NAME: &'static str = "LnxdriveConfirmPage";
        type Type = super::ConfirmPage;
        type ParentType = adw::NavigationPage;
    }

    impl ObjectImpl for ConfirmPage {}
    impl WidgetImpl for ConfirmPage {}
    impl NavigationPageImpl for ConfirmPage {}
}

glib::wrapper! {
    pub struct ConfirmPage(ObjectSubclass<imp::ConfirmPage>)
        @extends adw::NavigationPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl ConfirmPage {
    pub fn new(onboarding_view: &OnboardingView) -> Self {
        let page: Self = glib::Object::builder()
            .property("title", gettext("Ready to Sync"))
            .property("tag", "confirm")
            .build();

        page.imp()
            .onboarding_view
            .replace(Some(onboarding_view.clone()));

        page.build_ui();
        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        let ov = match imp.onboarding_view.borrow().clone() {
            Some(v) => v,
            None => return,
        };
        let state = ov.state();

        // Summary rows
        let account_email = state
            .account_email
            .clone()
            .unwrap_or_else(|| gettext("Unknown"));
        let sync_folder = state
            .sync_root
            .clone()
            .unwrap_or_else(|| gettext("Not selected"));

        let email_row = adw::ActionRow::builder()
            .title(&gettext("Account"))
            .subtitle(&account_email)
            .icon_name("avatar-default-symbolic")
            .build();

        let folder_row = adw::ActionRow::builder()
            .title(&gettext("Sync Folder"))
            .subtitle(&sync_folder)
            .icon_name("folder-symbolic")
            .build();

        let summary_group = adw::PreferencesGroup::new();
        summary_group.add(&email_row);
        summary_group.add(&folder_row);

        // "Start Syncing" button
        let start_button = gtk4::Button::builder()
            .label(&gettext("Start Syncing"))
            .halign(gtk4::Align::Center)
            .css_classes(["suggested-action", "pill"])
            .build();

        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .halign(gtk4::Align::Center)
            .margin_top(24)
            .build();
        button_box.append(&start_button);

        // Status page with check icon
        let status_page = adw::StatusPage::builder()
            .icon_name("emblem-ok-symbolic")
            .title(&gettext("All Set!"))
            .description(&gettext(
                "Your OneDrive account is ready. Review the details below and start syncing.",
            ))
            .build();

        let inner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(24)
            .build();
        inner.append(&summary_group);
        inner.append(&button_box);

        status_page.set_child(Some(&inner));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.set_content(Some(&status_page));

        self.set_child(Some(&toolbar_view));

        // Connect "Start Syncing" click
        let page = self.clone();
        start_button.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            page.on_start_syncing();
        });
    }

    /// Write config to daemon and trigger first sync.
    fn on_start_syncing(&self) {
        let imp = self.imp();

        let ov = match imp.onboarding_view.borrow().clone() {
            Some(v) => v,
            None => return,
        };

        let dbus_client = match ov.dbus_client().as_ref() {
            Some(c) => c.clone(),
            None => return,
        };

        let sync_root = ov.state().sync_root.clone().unwrap_or_default();
        let parent_window = ov.parent_window();

        glib::MainContext::default().spawn_local(async move {
            // Build a minimal YAML config pointing at the chosen sync root.
            let config_yaml = format!("sync_root: \"{}\"\n", sync_root);

            if let Err(e) = dbus_client.set_config(&config_yaml).await {
                if let Some(ref win) = parent_window {
                    let toast = adw::Toast::new(&format!(
                        "{}: {}",
                        gettext("Configuration error"),
                        e
                    ));
                    // Try to show toast via a ToastOverlay if available,
                    // otherwise fall back to showing the error in the window.
                    show_toast_on_window(win, &toast);
                }
                return;
            }

            if let Err(e) = dbus_client.sync_now().await {
                if let Some(ref win) = parent_window {
                    let toast = adw::Toast::new(&format!(
                        "{}: {}",
                        gettext("Could not start sync"),
                        e
                    ));
                    show_toast_on_window(win, &toast);
                }
                return;
            }

            // Success — switch to the preferences view.
            if let Some(ref win) = parent_window {
                win.show_preferences(&dbus_client);
            }
        });
    }
}

/// Helper: show a toast on the window. We wrap the window content in a
/// ToastOverlay if needed, then add the toast.
fn show_toast_on_window(window: &crate::window::LnxdriveWindow, toast: &adw::Toast) {
    let overlay = adw::ToastOverlay::new();
    if let Some(child) = window.content() {
        window.set_content(None::<&gtk4::Widget>);
        overlay.set_child(Some(&child));
    }
    window.set_content(Some(&overlay));
    overlay.add_toast(toast.clone());
}
