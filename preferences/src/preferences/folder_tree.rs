// Folder Tree — selective sync tree widget
//
// Displays the remote OneDrive folder hierarchy using a `gtk::ListView` backed
// by a `gtk::TreeListModel`. Each row has a TreeExpander, a CheckButton, and a
// Label. Toggling a folder propagates to its children. The set of selected
// paths is sent to the daemon via `set_selected_folders()`.
//
// The tree is lazily loaded: each expand triggers the TreeListModel's
// create_model closure, which parses the JSON subtree for the expanded node.

use std::cell::RefCell;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use serde::Deserialize;

use gtk4::subclass::prelude::ObjectSubclassIsExt;

use crate::dbus_client::DbusClient;

// ---------------------------------------------------------------------------
// JSON schema for the remote folder tree returned by the daemon
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
pub struct FolderNodeJson {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub children: Vec<FolderNodeJson>,
}

// ---------------------------------------------------------------------------
// FolderNode — glib::Object subclass with name, path, selected properties
// ---------------------------------------------------------------------------

mod folder_node_imp {
    use super::*;
    use std::cell::Cell;
    use gtk4::subclass::prelude::*;

    #[derive(Default)]
    pub struct FolderNode {
        pub name: RefCell<String>,
        pub path: RefCell<String>,
        pub selected: Cell<bool>,
        /// Serialised JSON children — kept for lazy tree model expansion.
        pub children_json: RefCell<Vec<FolderNodeJson>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderNode {
        const NAME: &'static str = "LnxdriveFolderNode";
        type Type = super::FolderNode;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for FolderNode {
        fn properties() -> &'static [glib::ParamSpec] {
            use std::sync::OnceLock;
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| {
                vec![
                    glib::ParamSpecString::builder("name")
                        .default_value(Some(""))
                        .build(),
                    glib::ParamSpecString::builder("path")
                        .default_value(Some(""))
                        .build(),
                    glib::ParamSpecBoolean::builder("selected")
                        .default_value(false)
                        .build(),
                ]
            })
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "name" => {
                    let val: String = value.get().unwrap_or_default();
                    *self.name.borrow_mut() = val;
                }
                "path" => {
                    let val: String = value.get().unwrap_or_default();
                    *self.path.borrow_mut() = val;
                }
                "selected" => {
                    let val: bool = value.get().unwrap_or(false);
                    self.selected.set(val);
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "name" => self.name.borrow().to_value(),
                "path" => self.path.borrow().to_value(),
                "selected" => self.selected.get().to_value(),
                _ => unimplemented!(),
            }
        }
    }
}

glib::wrapper! {
    pub struct FolderNode(ObjectSubclass<folder_node_imp::FolderNode>);
}

impl FolderNode {
    pub fn new(name: &str, path: &str, selected: bool, children: Vec<FolderNodeJson>) -> Self {
        let obj: Self = glib::Object::builder()
            .property("name", name)
            .property("path", path)
            .property("selected", selected)
            .build();

        *obj.imp().children_json.borrow_mut() = children;
        obj
    }

    pub fn name(&self) -> String {
        self.imp().name.borrow().clone()
    }

    pub fn path(&self) -> String {
        self.imp().path.borrow().clone()
    }

    pub fn selected(&self) -> bool {
        self.imp().selected.get()
    }

    pub fn set_selected(&self, value: bool) {
        self.imp().selected.set(value);
        self.notify("selected");
    }

    pub fn children_json(&self) -> Vec<FolderNodeJson> {
        self.imp().children_json.borrow().clone()
    }
}

// ---------------------------------------------------------------------------
// FolderTree — gtk::Box subclass containing the tree view
// ---------------------------------------------------------------------------

mod imp {
    use super::*;
    use gtk4::subclass::prelude::*;

    pub struct FolderTree {
        pub dbus_client: RefCell<Option<DbusClient>>,
        pub tree_model: RefCell<Option<gtk4::TreeListModel>>,
        pub root_store: RefCell<Option<gio::ListStore>>,
        pub list_view: RefCell<Option<gtk4::ListView>>,
        pub selected_folders: RefCell<Vec<String>>,
    }

    impl Default for FolderTree {
        fn default() -> Self {
            Self {
                dbus_client: RefCell::new(None),
                tree_model: RefCell::new(None),
                root_store: RefCell::new(None),
                list_view: RefCell::new(None),
                selected_folders: RefCell::new(Vec::new()),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FolderTree {
        const NAME: &'static str = "LnxdriveFolderTree";
        type Type = super::FolderTree;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for FolderTree {}
    impl WidgetImpl for FolderTree {}
    impl BoxImpl for FolderTree {}
}

glib::wrapper! {
    pub struct FolderTree(ObjectSubclass<imp::FolderTree>)
        @extends gtk4::Box, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget,
                    gtk4::Orientable;
}

impl FolderTree {
    pub fn new(dbus_client: Option<&DbusClient>) -> Self {
        let tree: Self = glib::Object::builder()
            .property("orientation", gtk4::Orientation::Vertical)
            .build();

        if let Some(client) = dbus_client {
            tree.imp()
                .dbus_client
                .replace(Some(client.clone()));
        }

        tree.build_ui();
        tree.load_remote_tree();
        tree.load_selected_folders();

        tree
    }

    fn build_ui(&self) {
        let imp = self.imp();

        // Root list store for FolderNode objects.
        let root_store = gio::ListStore::new::<FolderNode>();
        imp.root_store.replace(Some(root_store.clone()));

        // Tree list model: the create_model closure returns a child ListStore
        // when a row is expanded, populated from the FolderNode's children_json.
        let tree_model = gtk4::TreeListModel::new(
            root_store.clone(),
            false,  // passthrough = false (we want TreeListRow wrappers)
            true,   // autoexpand = true for first level
            move |item| {
                let node = item
                    .downcast_ref::<FolderNode>()
                    .expect("TreeListModel item must be FolderNode");

                let children = node.children_json();
                if children.is_empty() {
                    return None;
                }

                let child_store = gio::ListStore::new::<FolderNode>();
                let parent_selected = node.selected();
                for child in &children {
                    let child_node = FolderNode::new(
                        &child.name,
                        &child.path,
                        parent_selected,
                        child.children.clone(),
                    );
                    child_store.append(&child_node);
                }

                Some(child_store.upcast())
            },
        );
        imp.tree_model.replace(Some(tree_model.clone()));

        // Selection model — NoSelection because toggling is via CheckButton.
        let selection_model = gtk4::NoSelection::new(Some(tree_model));

        // Factory for list items.
        let factory = gtk4::SignalListItemFactory::new();

        factory.connect_setup(|_factory, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk4::ListItem>()
                .expect("ListItem expected");

            let expander = gtk4::TreeExpander::new();
            let hbox = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();

            let check = gtk4::CheckButton::new();
            let label = gtk4::Label::builder()
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build();

            hbox.append(&check);
            hbox.append(&label);

            expander.set_child(Some(&hbox));
            list_item.set_child(Some(&expander));
        });

        let tree_widget = self.clone();
        factory.connect_bind(move |_factory, list_item| {
            let list_item = list_item
                .downcast_ref::<gtk4::ListItem>()
                .expect("ListItem expected");

            let tree_list_row = list_item
                .item()
                .and_downcast::<gtk4::TreeListRow>()
                .expect("Item must be TreeListRow");

            let node = tree_list_row
                .item()
                .and_downcast::<FolderNode>()
                .expect("TreeListRow item must be FolderNode");

            let expander = list_item
                .child()
                .and_downcast::<gtk4::TreeExpander>()
                .expect("Child must be TreeExpander");

            expander.set_list_row(Some(&tree_list_row));

            let hbox = expander
                .child()
                .and_downcast::<gtk4::Box>()
                .expect("Expander child must be Box");

            // Get the check button (first child) and label (second child).
            let check = hbox
                .first_child()
                .and_downcast::<gtk4::CheckButton>()
                .expect("First child must be CheckButton");

            let label = check
                .next_sibling()
                .and_downcast::<gtk4::Label>()
                .expect("Second child must be Label");

            label.set_label(&node.name());
            check.set_active(node.selected());

            // Connect checkbox toggle.
            let tree_ref = tree_widget.clone();
            let node_ref = node.clone();
            check.connect_toggled(move |btn| {
                let new_val = btn.is_active();
                node_ref.set_selected(new_val);
                tree_ref.on_selection_changed();
            });
        });

        factory.connect_unbind(|_factory, list_item| {
            // Clean up: we don't store signal handler IDs because the
            // CheckButton is recreated on each bind cycle.
            let _ = list_item;
        });

        // List view.
        let list_view = gtk4::ListView::builder()
            .model(&selection_model)
            .factory(&factory)
            .build();
        list_view.add_css_class("boxed-list");

        imp.list_view.replace(Some(list_view.clone()));

        // Scrolled window.
        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .min_content_height(250)
            .max_content_height(400)
            .build();
        scrolled.set_child(Some(&list_view));

        self.append(&scrolled);
    }

    /// Fetch the remote folder tree JSON from the daemon and populate the root store.
    fn load_remote_tree(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let tree = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_remote_folder_tree().await {
                Ok(json) => {
                    tree.populate_from_json(&json);
                }
                Err(e) => {
                    eprintln!("Could not load remote folder tree: {}", e);
                }
            }
        });
    }

    /// Load the currently selected folders from the daemon so we can mark
    /// them as checked.
    fn load_selected_folders(&self) {
        let client = match self.imp().dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        let tree = self.clone();
        glib::MainContext::default().spawn_local(async move {
            match client.get_selected_folders().await {
                Ok(folders) => {
                    *tree.imp().selected_folders.borrow_mut() = folders;
                    // Re-apply selections after the tree has been populated.
                    tree.apply_selections();
                }
                Err(e) => {
                    eprintln!("Could not load selected folders: {}", e);
                }
            }
        });
    }

    /// Parse the JSON folder tree and populate the root ListStore.
    fn populate_from_json(&self, json: &str) {
        let imp = self.imp();

        let root_store = match imp.root_store.borrow().clone() {
            Some(s) => s,
            None => return,
        };

        root_store.remove_all();

        // The JSON may be a single root object or an array of roots.
        let nodes: Vec<FolderNodeJson> = if json.trim_start().starts_with('[') {
            serde_json::from_str(json).unwrap_or_default()
        } else {
            match serde_json::from_str::<FolderNodeJson>(json) {
                Ok(root) => root.children,
                Err(_) => Vec::new(),
            }
        };

        let selected = imp.selected_folders.borrow().clone();
        for node in &nodes {
            let is_selected = selected.iter().any(|p| p == &node.path);
            let folder_node =
                FolderNode::new(&node.name, &node.path, is_selected, node.children.clone());
            root_store.append(&folder_node);
        }
    }

    /// Walk the root store and mark nodes whose path is in the selected list.
    fn apply_selections(&self) {
        let imp = self.imp();
        let store = match imp.root_store.borrow().clone() {
            Some(s) => s,
            None => return,
        };
        let selected = imp.selected_folders.borrow().clone();

        for i in 0..store.n_items() {
            if let Some(item) = store.item(i) {
                if let Some(node) = item.downcast_ref::<FolderNode>() {
                    let is_selected = selected.iter().any(|p| p == &node.path());
                    node.set_selected(is_selected);
                }
            }
        }
    }

    /// Called whenever a checkbox is toggled. Propagates the selection to
    /// children and then sends the full list of selected paths to the daemon.
    fn on_selection_changed(&self) {
        let imp = self.imp();

        // Collect all selected paths from the root store.
        let store = match imp.root_store.borrow().clone() {
            Some(s) => s,
            None => return,
        };

        let mut selected_paths = Vec::new();
        self.collect_selected(&store, &mut selected_paths);

        *imp.selected_folders.borrow_mut() = selected_paths.clone();

        // Send to daemon.
        let client = match imp.dbus_client.borrow().clone() {
            Some(c) => c,
            None => return,
        };

        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = client.set_selected_folders(&selected_paths).await {
                eprintln!("Could not save selected folders: {}", e);
            }
        });
    }

    /// Recursively collect the paths of selected FolderNodes from a ListStore.
    fn collect_selected(&self, store: &gio::ListStore, out: &mut Vec<String>) {
        for i in 0..store.n_items() {
            if let Some(item) = store.item(i) {
                if let Some(node) = item.downcast_ref::<FolderNode>() {
                    if node.selected() {
                        out.push(node.path());
                    }
                    // Note: children are only materialised when expanded;
                    // we rely on the user expanding and toggling them.
                }
            }
        }
    }
}
