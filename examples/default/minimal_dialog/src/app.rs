// Demonstrates the built-in dialog system (feature = "tui").
//
// The runtime's modal payload type `M` is `DialogData<Purpose>`, so the focus
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
    /// Standard navigation actions provided by the keybinding presets.
    /// Activate is per-page — the handler decides what "enter" means for the
    /// currently focused target (here: open a dialog / quit).
    Nav(NavigationAction),
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
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

// The dialog content (`DialogData<Purpose>`) is the runtime's modal payload `M`.
// O = () — this example has no named simple overlays, only a modal (the dialog).
pub type App = TuiApp<View, Action, AppState, Handler, (), DialogData<Purpose>>;

pub struct Handler;

impl TuiActionHandler<View, Action, AppState, (), DialogData<Purpose>> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        state: &mut AppState,
        _runtime: RuntimeContext<'_, Action, (), DialogData<Purpose>>,
    ) -> Result<ActionOutcome<View, (), DialogData<Purpose>>, Self::Error> {
        Ok(match action {
            Action::Nav(NavigationAction::Activate) => match ctx.focus {
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
            Action::Nav(nav) => ActionOutcome::effect(nav.to_effect()),
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
        .page_fn(page_spec)
        .handler(Handler)
        // Vim preset covers focus movement (tab/arrows/hjkl), activate (enter),
        // leave section (esc), and quit (ctrl+c). The "Delete" action is the
        // app-specific payload that opens the dialog on activate.
        .vim_defaults()
        .build();
    app.refresh_page(&AppState::default());
    app
}
