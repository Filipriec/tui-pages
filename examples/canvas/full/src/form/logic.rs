use tui_pages::canvas;

use crate::app::{AppState, TotalComputer};

pub fn recompute_total(state: &mut AppState) {
    let mut computer = TotalComputer;
    state.form.recompute_all_fields(&mut computer);
}

pub fn message_for_result(result: canvas::ActionResult) -> String {
    match result {
        canvas::ActionResult::Success => "form updated".to_string(),
        canvas::ActionResult::Message(message) | canvas::ActionResult::Error(message) => message,
        _ => "form updated".to_string(),
    }
}
