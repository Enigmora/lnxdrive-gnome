// Onboarding Wizard — adw::Bin wrapping an adw::NavigationView
//
// A three-step wizard: AuthPage -> FolderPage -> ConfirmPage.
// Holds transient state (account info, chosen sync root) that is discarded
// on cancel and committed to the daemon on "Start Syncing".
//
// NavigationView is not subclassable in libadwaita-rs 0.7, so we use
// composition: OnboardingView is a Bin whose child is a NavigationView.

pub mod auth_page;
pub mod confirm_page;
pub mod folder_page;

use std::cell::RefCell;

use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;
use crate::window::LnxdriveWindow;

use auth_page::AuthPage;

// ---------------------------------------------------------------------------
// Transient onboarding state shared across pages
// ---------------------------------------------------------------------------

/// Mutable state accumulated during the onboarding flow.
/// Reset when the user cancels (FR-033).
#[derive(Clone, Debug, Default)]
pub struct OnboardingState {
    pub account_email: Option<String>,
    pub account_name: Option<String>,
    pub sync_root: Option<String>,
}

// ---------------------------------------------------------------------------
// OnboardingView — Bin wrapping a NavigationView (composition)
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct OnboardingView {
        pub nav_view: adw::NavigationView,
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub state: RefCell<OnboardingState>,
        pub parent_window: RefCell<Option<LnxdriveWindow>>,
    }

    impl Default for OnboardingView {
        fn default() -> Self {
            Self {
                nav_view: adw::NavigationView::new(),
                dbus_client: RefCell::new(None),
                state: RefCell::new(OnboardingState::default()),
                parent_window: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for OnboardingView {
        const NAME: &'static str = "LnxdriveOnboardingView";
        type Type = super::OnboardingView;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for OnboardingView {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().set_child(Some(&self.nav_view));
        }
    }
    impl WidgetImpl for OnboardingView {}
    impl BinImpl for OnboardingView {}
}

glib::wrapper! {
    pub struct OnboardingView(ObjectSubclass<imp::OnboardingView>)
        @extends adw::Bin, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl OnboardingView {
    /// Create the onboarding wizard and push the first page (auth).
    pub fn new(dbus_client: DbusClient, parent_window: LnxdriveWindow) -> Self {
        let view: Self = glib::Object::builder().build();

        {
            let imp = view.imp();
            *imp.dbus_client.borrow_mut() = Some(dbus_client);
            *imp.parent_window.borrow_mut() = Some(parent_window);
        }

        let auth_page = AuthPage::new(&view);
        view.nav_view().push(&auth_page);

        view
    }

    /// Access the inner NavigationView for push/pop operations.
    pub fn nav_view(&self) -> &adw::NavigationView {
        &self.imp().nav_view
    }

    /// Borrow the shared D-Bus client.
    pub fn dbus_client(&self) -> std::cell::Ref<'_, Option<DbusClient>> {
        self.imp().dbus_client.borrow()
    }

    /// Borrow the mutable onboarding state.
    pub fn state(&self) -> std::cell::Ref<'_, OnboardingState> {
        self.imp().state.borrow()
    }

    /// Mutably borrow the onboarding state.
    pub fn state_mut(&self) -> std::cell::RefMut<'_, OnboardingState> {
        self.imp().state.borrow_mut()
    }

    /// Get a reference to the parent LnxdriveWindow.
    pub fn parent_window(&self) -> Option<LnxdriveWindow> {
        self.imp().parent_window.borrow().clone()
    }

    /// Cancel onboarding: reset all transient state and pop to the first page (FR-033).
    pub fn on_cancel(&self) {
        *self.imp().state.borrow_mut() = OnboardingState::default();
        self.nav_view().pop_to_tag("auth");
    }
}
