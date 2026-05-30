// Help is a static page, so it's just wiring + drawing — no logic file.
mod page;
pub mod ui;

pub use page::{handle, page_spec};
