use tui_pages::canvas;

use crate::app::AppState;

// The page's two buttons.
pub const BUTTONS: [&str; 2] = ["Clear", "Go to Form"];

pub fn mode_label(mode: canvas::AppMode) -> &'static str {
    match mode {
        canvas::AppMode::Edit => "INSERT",
        canvas::AppMode::ReadOnly => "NORMAL",
        canvas::AppMode::Highlight => "VISUAL",
        canvas::AppMode::Command => "COMMAND",
        canvas::AppMode::General => "GENERAL",
    }
}

/// The "Clear" button's effect: empty the textarea (back in NORMAL, un-entered).
pub fn clear_textarea(state: &mut AppState) {
    let mut body = canvas::TextAreaState::from_text("");
    body.use_wrap();
    state.body = body;
    state.entered = false;
    state.message = "textarea cleared".to_string();
}
