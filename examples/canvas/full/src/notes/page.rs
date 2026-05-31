use tui_pages::canvas;
use tui_pages::prelude::*;

use crate::app::{Action, AppState, View};

pub fn page_spec(_state: &AppState, focus: Option<&FocusTarget>) -> PageSpec {
    let spec = PageSpec::new().focus(PageFocusBuilder::new().canvas_field(0).button(0));
    if matches!(focus, Some(FocusTarget::Button(0))) {
        spec
    } else {
        spec.canvas_mode(canvas::AppMode::Edit)
    }
}

pub fn handle(
    action: Action,
    ctx: &ActionContext<View>,
    _state: &mut AppState,
) -> ActionOutcome<View> {
    match action {
        Action::Select => match ctx.focus {
            Some(FocusTarget::Button(0)) => {
                ActionOutcome::effect(TuiEffect::Navigate(View::Search))
            }
            _ => ActionOutcome::none(),
        },
        _ => ActionOutcome::none(),
    }
}
