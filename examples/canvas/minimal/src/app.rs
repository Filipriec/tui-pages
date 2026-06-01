//! Everything that talks to `tui-pages`

use crate::{clear_form, State};
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Form,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    FocusNext,
    FocusPrev,
    Activate,
    Quit,
}

pub struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
            Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
            Action::Activate => match _ctx.focus {
                Some(FocusTarget::Button(0)) => {
                    clear_form(state);
                    ActionOutcome::none()
                }
                Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
        })
    }
}

fn page_spec(_view: &View, _state: &State, _focus: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new().focus(
        PageFocusBuilder::new()
            // Register one CanvasField per editor field so the two form fields
            // are explicit in the focus list: [CanvasField(0), CanvasField(1),
            // Button(0), Button(1)]. The editor still owns movement *between* the
            // two fields (focus collapses all canvas targets into one stop and
            // hands off at the boundary), so the editor remains the source of
            // truth for which field is active.
            .canvas_fields(2)
            .button(0)
            .button(1),
    )
}

pub fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Form)
        .page_fn(page_spec)
        .handler(Handler)
        // Attach the form editor - it handles canvas actions internally
        // so they never reach our Action type.
        .canvas_form_editor(0)
        // Navigation is a flat line of four stops — field 0, field 1, Clear, Quit.
        // On a button (general mode) the page navigates with Tab/Enter, plus
        // vim keys so j/k/h/l flow continues straight off the canvas: `k` on the
        // first button re-enters the form (its last field), `j`/`l` step to the
        // next button. Inside the canvas, `canvas_defaults` already bound j/k to
        // field movement, and the editor hands off at the field boundary.
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "backtab", Action::FocusPrev)
        .bind(modes::GENERAL, "j", Action::FocusNext)
        .bind(modes::GENERAL, "k", Action::FocusPrev)
        .bind(modes::GENERAL, "l", Action::FocusNext)
        .bind(modes::GENERAL, "h", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Activate)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build()
}
