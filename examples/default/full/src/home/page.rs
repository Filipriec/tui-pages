use tui_pages::prelude::*;

use super::logic;
use crate::app::{Action, AppState, Overlay, View};

pub fn page_spec(_state: &AppState) -> PageSpec<Overlay> {
    PageSpec::new()
        .focus(PageFocusBuilder::new().button(0).button(1))
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn handle(
    action: Action,
    ctx: &ActionContext<View, Overlay>,
    _state: &mut AppState,
) -> ActionOutcome<View, Overlay> {
    match action {
        Action::Nav(NavigationAction::Activate) => match &ctx.focus {
            Some(FocusTarget::Button(i)) => match logic::destination(*i) {
                Some(view) => ActionOutcome::effect(TuiEffect::Navigate(view)),
                None => ActionOutcome::none(),
            },
            _ => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Activate)),
        },
        _ => ActionOutcome::none(),
    }
}
