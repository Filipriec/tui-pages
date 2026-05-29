// Demonstrates the built-in dialog system (feature = "dialog").
//
// The runtime's dialog payload type `D` is `DialogData<Purpose>`, so the focus
// manager stores the dialog content and tracks the active button. The handler
// only opens the dialog; the modal itself is driven in the event loop (main.rs)
// using the `dialog::*` helpers.

use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
}

/// Application-owned "which dialog is this" payload. Opaque to the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    FocusNext,
    FocusPrev,
    Select,
    Quit,
}

#[derive(Debug)]
pub struct AppState {
    pub items: Vec<String>,
    pub message: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            items: vec!["Apples".into(), "Bananas".into(), "Cherries".into()],
            message: "Highlight \"Delete an item\" and press Enter.".into(),
        }
    }
}

// The dialog payload (`DialogData<Purpose>`) is the runtime's `D` type param.
pub type App = TuiPages<View, Action, AppState, PageFn<View, AppState>, Handler, DialogData<Purpose>>;

pub struct Handler;

impl TuiActionHandler<View, Action, AppState, DialogData<Purpose>> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<View, DialogData<Purpose>>, Self::Error> {
        Ok(match action {
            Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),

            Action::Select => match ctx.focus {
                // "Delete an item" — open a confirmation dialog.
                Some(FocusTarget::Button(0)) => {
                    if let Some(first) = state.items.first() {
                        let dialog = DialogData::new(
                            "Delete an item?",
                            format!("Delete \"{first}\"?\nThis cannot be undone."),
                            ["Delete", "Cancel"],
                            Purpose::ConfirmDelete,
                        );
                        // `show_intent` turns the dialog into a focus effect.
                        ActionOutcome::effect(TuiEffect::Focus(dialog.show_intent()))
                    } else {
                        state.message = "No items left to delete.".into();
                        ActionOutcome::none()
                    }
                }
                // "Quit"
                Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
        })
    }
}

fn page_spec(_view: &View, _state: &AppState, _focus: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus_targets(PageFocusBuilder::new().button(0).button(1).build())
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn build() -> App {
    let mut app = TuiPages::builder(View::Main)
        .pages(page_spec as PageFn<View, AppState>)
        .handler(Handler)
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build();
    app.refresh_page(&AppState::default());
    app
}
