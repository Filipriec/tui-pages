//! Everything that talks to `tui-pages`
//!
//! The textarea is a single top-level focus stop. By default `j`/`k` treat it
//! as one stop and step straight to the buttons; you press Enter to *enter* it,
//! and only then do the modal `nor`/`ins` keys move the cursor line-by-line.
//! `Esc` in NORMAL leaves it again.
//!
//! The `canvas_textarea_widget` builder handles enter/edit/exit flow internally.
//! INSERT mode typing is handled in `main.rs` by the textarea's own editor.

use crate::{clear_textarea, State};
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Editor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Standard navigation actions provided by the keybinding presets.
    /// Activate is per-page — the handler decides what "enter" means for the
    /// currently focused target (here: clear textarea / quit).
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
            // Enter: activate the focused button. The textarea widget builder
            // handles Enter on the textarea to enter edit mode.
            Action::Nav(NavigationAction::Activate) => match ctx.focus {
                Some(FocusTarget::Button(0)) => {
                    clear_textarea(state);
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
    // One canvas field for the single textarea, then the two buttons:
    // [CanvasField(0), Button(0), Button(1)].
    PageSpec::new().focus(
        PageFocusBuilder::new()
            .canvas_field(0)
            .button(0)
            .button(1),
    )
}

pub fn build() -> TuiApp<View, Action, State, Handler, (), (), CanvasHooks> {
    TuiPages::builder(View::Editor)
        .page_fn(page_spec)
        .handler(Handler)
        // Attach the textarea widget - it handles enter/edit/exit flow internally.
        // Canvas actions (i/a for modes, j/k/h/l for movement) are handled by
        // the builder and never reach our Action type.
        .canvas_textarea_widget(0)
        // Vim preset covers button navigation and the un-entered textarea as a
        // single focus stop: j/k/h/l + Tab/Backtab, Enter to activate, Ctrl-C
        // to quit. The widget builder turns j/k into a canvas-boundary exit.
        .vim_defaults()
        .build()
}
