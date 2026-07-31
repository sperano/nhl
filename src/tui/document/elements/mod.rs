//! Document elements that can be rendered in a document
//!
//! Provides various element types (text, headings, tables, links, etc.)
//! that can be composed to build documents.
//!
//! - `types`: `DocTabDef`, `RowAlignment`, and layout constants
//! - `element`: the `DocumentElement` enum and its `Debug` impl
//! - `behavior`: `height()`, `collect_focusable()`, and `render()`
//! - `constructors`, `table_constructor`, `row_constructor`: builder methods
//! - `render`: text/heading/link/separator/group rendering primitives
//! - `render_layout`: row layout and buffer-clipping helpers
//! - `render_composites`: table-backed composites (team boxscore, tabs)

mod behavior;
mod constructors;
mod element;
mod render;
mod render_composites;
mod render_layout;
mod row_constructor;
mod table_constructor;
mod types;

pub use element::DocumentElement;
pub use render::TEAM_BOXSCORE_SIDE_BY_SIDE_WIDTH;
pub use types::{DocTabDef, RowAlignment, TAB_BAR_HEIGHT};
