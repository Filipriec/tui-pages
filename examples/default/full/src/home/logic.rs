use crate::app::View;

// Home's two buttons and where each one goes. Keeping this here means the page
// and its rendering can't disagree about the labels or the order.
pub const BUTTONS: [&str; 2] = ["Open Notes", "Open Help"];

pub fn destination(button: usize) -> Option<View> {
    match button {
        0 => Some(View::Notes),
        1 => Some(View::Help),
        _ => None,
    }
}
