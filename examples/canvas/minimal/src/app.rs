//! Everything that talks to `tui-pages`

use crate::{clear_form, State};
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Form,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Standard navigation actions provided by the keybinding presets.
    /// Activate is per-page — the handler decides what "enter" means for the
    /// currently focused target (here: clear form / quit).
    Nav(NavigationAction),
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
}

pub struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        state: &mut State,
        _runtime: RuntimeContext<'_, Action>,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::Nav(NavigationAction::Activate) => match ctx.focus {
                Some(FocusTarget::Button(0)) => {
                    clear_form(state);
                    ActionOutcome::none()
                }
                Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
            Action::Nav(nav) => ActionOutcome::effect(nav.to_effect()),
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
        // Vim preset covers the flat focus line (field 0, field 1, Clear, Quit):
        // j/k/h/l plus Tab/Backtab on the page, Enter to activate, Ctrl-C to quit.
        // Inside the form editor, j/k move between fields and the editor hands
        // off at the field boundary — the form editor's own pipeline swallows
        // those keys before they reach our Action type.
        .vim_defaults()
        .build()
}
