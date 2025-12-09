pub mod data_loading;
pub mod document_stack;
pub mod navigation;
pub mod settings;
pub mod standings;

pub use data_loading::reduce_data_loading;
pub use document_stack::reduce_document_stack;
pub use navigation::reduce_navigation;
pub use settings::reduce_settings;
pub use standings::rebuild_standings_focusable_metadata;
