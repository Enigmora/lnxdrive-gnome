// Folder Page — second step of the onboarding wizard
//
// Lets the user choose the local sync root (defaults to ~/OneDrive).
// "Continue" validates the path and pushes the ConfirmPage.
// "Back" pops back to the AuthPage.

use std::cell::RefCell;
use std::path::PathBuf;

use gettextrs::gettext;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use super::confirm_page::ConfirmPage;
use super::OnboardingView;

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;
    use libadwaita::subclass::prelude::*;

    pub struct FolderPage {
        pub onboarding_view: RefCell<Option<OnboardingView>>,
        pub selected_path: RefCell<PathBuf>,
        pub path_row: RefCell<Option<adw::ActionRow>>,
    }

    impl Default for FolderPage {
        fn default() -> Self {
            let default_path = glib::home_dir().join("OneDrive");
            Self {
                onboarding_view: RefCell::new(None),
                selected_path: RefCell::new(default_path),
                path_row: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderPage {
        const NAME: &'static str = "LnxdriveFolderPage";
        type Type = super::FolderPage;
        type ParentType = adw::NavigationPage;
    }

    impl ObjectImpl for FolderPage {}
    impl WidgetImpl for FolderPage {}
    impl NavigationPageImpl for FolderPage {}
}

glib::wrapper! {
    pub struct FolderPage(ObjectSubclass<imp::FolderPage>)
        @extends adw::NavigationPage, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl FolderPage {
    pub fn new(onboarding_view: &OnboardingView) -> Self {
        let page: Self = glib::Object::builder()
            .property("title", gettext("Choose Folder"))
            .property("tag", "folder")
            .build();

        page.imp()
            .onboarding_view
            .replace(Some(onboarding_view.clone()));

        page.build_ui();
        page
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // Path display row
        let initial_path = imp.selected_path.borrow().display().to_string();
        let path_row = adw::ActionRow::builder()
            .title(&gettext("Sync Folder"))
            .subtitle(&initial_path)
            .build();

        // "Choose Folder..." button as a suffix
        let choose_button = gtk4::Button::builder()
            .icon_name("folder-open-symbolic")
            .tooltip_text(&gettext("Choose Folder..."))
            .valign(gtk4::Align::Center)
            .css_classes(["flat"])
            .build();
        path_row.add_suffix(&choose_button);
        path_row.set_activatable_widget(Some(&choose_button));

        imp.path_row.replace(Some(path_row.clone()));

        let prefs_group = adw::PreferencesGroup::builder()
            .title(&gettext("Sync Location"))
            .description(&gettext(
                "Choose where OneDrive files will be stored on your computer.",
            ))
            .build();
        prefs_group.add(&path_row);

        // Action buttons
        let continue_button = gtk4::Button::builder()
            .label(&gettext("Continue"))
            .halign(gtk4::Align::Center)
            .css_classes(["suggested-action", "pill"])
            .build();

        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .halign(gtk4::Align::Center)
            .margin_top(24)
            .build();
        button_box.append(&continue_button);

        // Outer layout
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(24)
            .margin_start(24)
            .margin_end(24)
            .margin_top(24)
            .margin_bottom(24)
            .valign(gtk4::Align::Center)
            .build();
        content.append(&prefs_group);
        content.append(&button_box);

        // Clamp for responsive width
        let clamp = adw::Clamp::builder()
            .maximum_size(500)
            .child(&content)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&adw::HeaderBar::new());
        toolbar_view.set_content(Some(&clamp));

        self.set_child(Some(&toolbar_view));

        // Connect "Choose Folder..." button
        let page = self.clone();
        choose_button.connect_clicked(move |_| {
            page.on_choose_folder();
        });

        // Connect "Continue" button
        let page = self.clone();
        continue_button.connect_clicked(move |_| {
            page.on_continue();
        });
    }

    /// Open a folder chooser dialog.
    fn on_choose_folder(&self) {
        let dialog = gtk4::FileDialog::builder()
            .title(&gettext("Choose Sync Folder"))
            .modal(true)
            .build();

        // Set initial folder to current selection
        let current = self.imp().selected_path.borrow().clone();
        if current.exists() {
            let file = gtk4::gio::File::for_path(&current);
            dialog.set_initial_folder(Some(&file));
        }

        let page = self.clone();
        let parent_win: Option<gtk4::Window> = self
            .imp()
            .onboarding_view
            .borrow()
            .as_ref()
            .and_then(|ov| ov.parent_window())
            .map(|w| w.upcast::<gtk4::Window>());

        dialog.select_folder(
            parent_win.as_ref(),
            None::<&gtk4::gio::Cancellable>,
            move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        page.set_selected_path(path);
                    }
                }
                // User cancelled — do nothing.
            },
        );
    }

    /// Update the selected path and refresh the UI.
    fn set_selected_path(&self, path: PathBuf) {
        let display = path.display().to_string();
        *self.imp().selected_path.borrow_mut() = path;

        if let Some(ref row) = *self.imp().path_row.borrow() {
            row.set_subtitle(&display);
        }
    }

    /// Validate and proceed to the confirm page.
    fn on_continue(&self) {
        let imp = self.imp();
        let path = imp.selected_path.borrow().clone();

        // Store in onboarding state
        if let Some(ref ov) = *imp.onboarding_view.borrow() {
            {
                let mut state = ov.state_mut();
                state.sync_root = Some(path.display().to_string());
            }

            let confirm_page = ConfirmPage::new(ov);
            ov.nav_view().push(&confirm_page);
        }
    }
}
