// Conflicts Module
//
// Provides UI components for conflict detection and resolution:
// - ConflictDetailDialog: side-by-side details with resolution options
// - ConflictListPage: lists all unresolved conflicts with batch actions

pub mod conflict_dialog;
pub mod conflict_list;

pub use conflict_list::ConflictListPage;
