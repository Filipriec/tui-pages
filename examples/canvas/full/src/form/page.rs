use tui_pages::prelude::*;

use super::logic;
use crate::app::{Action, AppState, Overlay, Purpose, View};

pub fn page_spec(_state: &AppState) -> PageSpec<Overlay> {
    // Two canvas fields for the form, then the three buttons:
    // [CanvasField(0), CanvasField(1), Button(0), Button(1), Button(2)]. The
    // editor still owns movement *between* its two fields; focus collapses the
    // canvas targets into one stop and hands off at the boundary.
    PageSpec::new().focus(
        PageFocusBuilder::new()
            .canvas_fields(2)
            .button(0)
            .button(1)
            .button(2),
    )
}

pub fn handle(
    action: Action,
    ctx: &ActionContext<View, Overlay>,
    state: &mut AppState,
) -> ActionOutcome<View, Overlay, DialogData<Purpose>> {
    match action {
        Action::Select => match ctx.focus {
            // Login: open a modal dialog previewing the data being posted.
            // `show_intent` turns the dialog into a focus effect; the focus
            // manager owns it until the event loop resolves it.
            Some(FocusTarget::Button(0)) => {
                ActionOutcome::effect(TuiEffect::Focus(logic::login_dialog(state).show_intent()))
            }
            Some(FocusTarget::Button(1)) => {
                ActionOutcome::effect(TuiEffect::Navigate(View::Editor))
            }
            Some(FocusTarget::Button(2)) => ActionOutcome::effect(TuiEffect::Navigate(View::Help)),
            _ => ActionOutcome::none(),
        },
        _ => ActionOutcome::none(),
    }
}
