use tui_pages::canvas;
use tui_pages::prelude::*;

use super::logic;
use crate::app::{Action, AppState, View};

pub fn page_spec(state: &AppState, focus: Option<&FocusTarget>) -> PageSpec {
    let spec = PageSpec::new().focus(PageFocusBuilder::new().canvas_field(0).button(0));
    if matches!(focus, Some(FocusTarget::Button(0))) {
        spec
    } else {
        spec.canvas_editor(&state.form)
    }
}

pub fn handle(
    action: Action,
    ctx: &ActionContext<View>,
    state: &mut AppState,
) -> ActionOutcome<View> {
    match action {
        Action::Canvas(action) => match canvas::dispatch_action(&mut state.form, action) {
            canvas::CanvasDispatchOutcome::Focus(intent) => {
                ActionOutcome::effect(TuiEffect::Focus(intent))
            }
            canvas::CanvasDispatchOutcome::Applied(result) => {
                logic::recompute_total(state);
                state.message = logic::message_for_result(result);
                ActionOutcome::none()
            }
        },
        Action::Select => match ctx.focus {
            Some(FocusTarget::Button(0)) => ActionOutcome::effect(TuiEffect::Navigate(View::Notes)),
            _ => ActionOutcome::none(),
        },
        _ => ActionOutcome::none(),
    }
}
