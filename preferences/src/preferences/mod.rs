// Preferences Dialog — adw::PreferencesDialog subclass
//
// A three-page preferences panel: Account, Sync, and Advanced.
// Each page is an adw::PreferencesPage subclass that reads from and writes to
// the LNXDrive daemon via the shared DbusClient.

pub mod account_page;
pub mod advanced_page;
pub mod folder_tree;
pub mod sync_page;

use std::cell::RefCell;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;

use account_page::AccountPage;
use advanced_page::AdvancedPage;
use sync_page::SyncPage;

use crate::conflicts::ConflictListPage;

// ---------------------------------------------------------------------------
// PreferencesDialog — adw::PreferencesDialog subclass
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct PreferencesDialog {
        pub dbus_client: RefCell<Option<DbusClient>>,
    }

    impl Default for PreferencesDialog {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesDialog {
        const NAME: &'static str = "LnxdrivePreferencesDialog";
        type Type = super::PreferencesDialog;
        type ParentType = adw::PreferencesDialog;
    }

    impl ObjectImpl for PreferencesDialog {}
    impl WidgetImpl for PreferencesDialog {}
    impl AdwDialogImpl for PreferencesDialog {}
    impl PreferencesDialogImpl for PreferencesDialog {}
}

glib::wrapper! {
    pub struct PreferencesDialog(ObjectSubclass<imp::PreferencesDialog>)
        @extends adw::PreferencesDialog, adw::Dialog, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl PreferencesDialog {
    /// Create the preferences dialog and populate it with the four pages.
    /// If `initial_page` matches a page name, navigate to it.
    pub fn new(dbus_client: &DbusClient, initial_page: Option<&str>) -> Self {
        let dialog: Self = glib::Object::builder()
            .property("title", gettext("LNXDrive Preferences"))
            .property("search-enabled", true)
            .build();

        dialog
            .imp()
            .dbus_client
            .replace(Some(dbus_client.clone()));

        // Build the four pages.
        let account_page = AccountPage::new(dbus_client);
        let sync_page = SyncPage::new(dbus_client);
        let conflicts_page = ConflictListPage::new(dbus_client);
        let advanced_page = AdvancedPage::new(dbus_client);

        dialog.add(&account_page);
        dialog.add(&sync_page);
        dialog.add(&conflicts_page);
        dialog.add(&advanced_page);

        // Navigate to initial page if specified
        if let Some(page_name) = initial_page {
            match page_name {
                "account" => dialog.set_visible_page(&account_page),
                "sync" => dialog.set_visible_page(&sync_page),
                "conflicts" => dialog.set_visible_page(&conflicts_page),
                "advanced" => dialog.set_visible_page(&advanced_page),
                _ => {}
            }
        }

        dialog
    }

    /// Present the dialog over the given parent widget.
    pub fn present(&self, parent: &impl IsA<gtk4::Widget>) {
        adw::prelude::AdwDialogExt::present(self, Some(parent));
    }
}
