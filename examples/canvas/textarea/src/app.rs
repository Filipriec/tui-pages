//! Everything that talks to `tui-pages`
//!
//! The textarea is a single top-level focus stop. By default `j`/`k` treat it
//! as one stop and step straight to the buttons; you press Enter to *enter* it
//! (`state.in_textarea`), and only then do the modal `nor`/`ins` keys move the
//! cursor line-by-line. `Esc` in NORMAL leaves it again.
//!
//! A `TextAreaState` derefs to the same modal `FormEditor` the form example
//! uses (`state.body.editor_mut()`), which is what gives it real nor/ins modes.
//! INSERT-mode typing is handled in `main.rs` by the textarea's own editor.

use crate::{clear_textarea, State};
use tui_pages::canvas;
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Editor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Canvas(canvas::CanvasAction),
    FocusNext,
    FocusPrev,
    Activate,
    ExitTextarea,
    Quit,
}

impl From<canvas::CanvasAction> for Action {
    fn from(action: canvas::CanvasAction) -> Self {
        Self::Canvas(action)
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
        let on_canvas = ctx.focus.as_ref().is_some_and(FocusTarget::is_canvas);
        Ok(match action {
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
            // From a button, step to the next/prev target. From the (un-entered)
            // textarea, `Next`/`Prev` are no-ops, so leave it via the canvas
            // boundary instead — that is how you step off a canvas stop.
            Action::FocusNext => {
                let intent = if on_canvas {
                    FocusIntent::ExitCanvasForward
                } else {
                    FocusIntent::Next
                };
                ActionOutcome::effect(TuiEffect::Focus(intent))
            }
            Action::FocusPrev => {
                let intent = if on_canvas {
                    FocusIntent::ExitCanvasBackward
                } else {
                    FocusIntent::Prev
                };
                ActionOutcome::effect(TuiEffect::Focus(intent))
            }
            // Enter: activate the focused button, or enter the textarea.
            Action::Activate => match ctx.focus {
                Some(FocusTarget::Button(0)) => {
                    clear_textarea(state);
                    ActionOutcome::none()
                }
                Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Quit),
                Some(FocusTarget::CanvasField(_)) => {
                    state.in_textarea = true;
                    ActionOutcome::none()
                }
                _ => ActionOutcome::none(),
            },
            // Esc in NORMAL leaves the textarea back to the top-level stop.
            Action::ExitTextarea => {
                state.in_textarea = false;
                ActionOutcome::none()
            }
            // NORMAL-mode canvas actions run on the textarea's underlying editor
            // once entered. We pass `allow_exit_in_read_only = false` so j/k
            // clamp at the first/last line instead of jumping to the buttons —
            // leaving the textarea is an explicit Esc (see `ExitTextarea`).
            Action::Canvas(action) => {
                let _ = canvas::execute_action_for_host_with_options(
                    state.body.editor_mut(),
                    action,
                    false,
                );
                ActionOutcome::none()
            }
        })
    }
}

fn page_spec(_view: &View, state: &State, focus: Option<&FocusTarget>) -> PageSpec {
    let spec = PageSpec::new().focus(
        PageFocusBuilder::new()
            // One canvas field for the single textarea, then the two buttons:
            // [CanvasField(0), Button(0), Button(1)].
            .canvas_field(0)
            .button(0)
            .button(1),
    );
    // Only mirror the editor's nor/ins modes once the textarea is *entered*.
    // Otherwise (buttons, or the textarea as a plain top-level stop) the page is
    // in general mode, where j/k step between stops and Enter selects.
    match focus {
        Some(FocusTarget::CanvasField(_)) if state.in_textarea => {
            spec.canvas_editor(state.body.editor())
        }
        _ => spec,
    }
}

pub fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Editor)
        .page_fn(page_spec)
        .handler(Handler)
        // Modal canvas keymaps (NORMAL i/a, j/k/h/l, o/O, …). Active only while
        // the textarea is entered. INSERT keys are handled in main.rs.
        .canvas_defaults()
        // Esc in NORMAL exits the textarea back to the top-level stop.
        .bind(modes::NORMAL, "esc", Action::ExitTextarea)
        // Top-level / button navigation (general mode). On a button these step
        // between buttons; on the un-entered textarea the handler turns them
        // into a canvas-boundary exit so j/k treat it as a single stop.
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
