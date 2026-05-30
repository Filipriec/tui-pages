use tui_pages::prelude::*;

use crate::app::{Action, AppState, Overlay, View};

// Help has nothing to focus and no actions of its own — it only declares the
// modes its keys live in.
pub fn page_spec(_state: &AppState) -> PageSpec<Overlay> {
    PageSpec::new().modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn handle(
    _action: Action,
    _ctx: &ActionContext<View, Overlay>,
    _state: &mut AppState,
) -> ActionOutcome<View, Overlay> {
    ActionOutcome::none()
}
