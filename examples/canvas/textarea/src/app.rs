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
            // Enter: activate the focused button. The textarea widget builder
            // handles Enter on the textarea to enter edit mode.
            Action::Activate => match _ctx.focus {
                Some(FocusTarget::Button(0)) => {
                    clear_textarea(state);
                    ActionOutcome::none()
                }
                Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
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

pub fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Editor)
        .page_fn(page_spec)
        .handler(Handler)
        // Attach the textarea widget - it handles enter/edit/exit flow internally.
        // Canvas actions (i/a for modes, j/k/h/l for movement) are handled by
        // the builder and never reach our Action type.
        .canvas_textarea_widget(
            0, // focus index
            |state: &mut State| &mut state.body,
            |state: &mut State| &mut state.entered,
        )
        // Top-level / button navigation (general mode). On a button these step
        // between buttons; on the un-entered textarea the widget builder turns
        // j/k into a canvas-boundary exit so they treat it as a single stop.
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