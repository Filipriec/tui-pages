// Notes page: page.rs wires it to tui-pages, logic.rs holds the note list and
// selection, ui.rs draws it.
mod logic;
mod page;
pub mod ui;

pub use page::{handle, page_spec};
