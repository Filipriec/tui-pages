use tui_pages::prelude::*;

use super::logic;
use crate::app::{Action, AppState, Overlay, Purpose, View};

pub fn page_spec(_state: &AppState) -> PageSpec<Overlay> {
    // One canvas field for the textarea, then the two buttons:
    // [CanvasField(0), Button(0), Button(1)]. The textarea is a single stop:
    // j/k step past it to the buttons, Enter *enters* it for inner navigation.
    PageSpec::new().focus(
        PageFocusBuilder::new()
            .canvas_field(0)
            .button(0)
            .button(1),
    )
}

pub fn handle(
    action: Action,
    ctx: &ActionContext<View, Overlay>,
    state: &mut AppState,
) -> ActionOutcome<View, Overlay, DialogData<Purpose>> {
    match action {
        // Enter on a button activates it. Enter on the textarea is handled by the
        // widget builder (it enters edit mode) and never reaches here.
        Action::Select => match ctx.focus {
            Some(FocusTarget::Button(0)) => {
                logic::clear_textarea(state);
                ActionOutcome::none()
            }
            Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Navigate(View::Form)),
            _ => ActionOutcome::none(),
        },
        _ => ActionOutcome::none(),
    }
}
