//! Everything that talks to `tui-pages`.
//!
//! A single-line `TextInput` is registered as a canvas widget. As you type, the
//! runtime asks the app for an inline completion suffix
//! (`canvas_textinput_suggestion_suffix`) and renders it as ghost text; `Tab`
//! accepts it. The widget builder handles enter/edit/exit flow internally.

use crate::{clear_input, State};
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Input,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Standard navigation actions provided by the keybinding presets.
    /// Activate is per-page — the handler decides what "enter" means for the
    /// currently focused target (here: clear input / quit).
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
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            // Enter on a button activates it. Enter on the input is handled by the
            // widget builder (it enters edit mode) and never reaches here.
            Action::Nav(NavigationAction::Activate) => match ctx.focus {
                Some(FocusTarget::Button(0)) => {
                    clear_input(state);
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
    // One canvas field for the input, then the two buttons:
    // [CanvasField(0), Button(0), Button(1)].
    PageSpec::new().focus(PageFocusBuilder::new().canvas_field(0).button(0).button(1))
}

pub fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Input)
        .page_fn(page_spec)
        .handler(Handler)
        // Attach the text input widget - it handles enter/edit/exit and the
        // inline suggestion suffix internally.
        .canvas_textinput_widget(0)
        // Vim preset covers the focus line: j/k/h/l on the un-entered input
        // (a single stop) and stepping through the buttons, Tab/Backtab,
        // Enter to activate, Ctrl-C to quit.
        .vim_defaults()
        .build()
}
