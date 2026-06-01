use tui_pages::prelude::*;

use crate::app::{AppState, Purpose};

// The page's three buttons. Keeping the labels here means the page and its
// rendering can't disagree about their order.
pub const BUTTONS: [&str; 3] = ["Login", "Go to Editor", "Help"];

/// Build the confirmation dialog shown when "Login" is pressed. It previews the
/// data that would be posted, straight from the form's current field values.
pub fn login_dialog(state: &AppState) -> DialogData<Purpose> {
    let contact = state.form.data_provider();
    let name = contact.values.first().map(String::as_str).unwrap_or("");
    let email = contact.values.get(1).map(String::as_str).unwrap_or("");

    DialogData::new(
        "Confirm login",
        format!("POST /login\n\nName:  {name}\nEmail: {email}"),
        ["Post", "Cancel"],
        Purpose::PostLogin,
    )
}
