// Editor page: page.rs wires it to tui-pages, logic.rs holds the button labels
// and the Clear side effect, ui.rs draws the TextArea and the buttons.
mod logic;
mod page;
pub mod ui;

pub use page::{handle, page_spec};
