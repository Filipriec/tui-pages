// The wiring layer. Shared types, the key bindings, and two routers that fan
// out to the per-page modules. No page logic lives here — each page owns that in
// its own folder (home/, notes/, help/).

use tui_pages::prelude::*;

use crate::{help, home, notes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overlay {
    CommandBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Home,
    Notes,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Standard navigation actions provided by the keybinding presets
    /// (focus movement, leave section, quit, buffer/pane splits, etc.).
    Nav(NavigationAction),
    GotoHome,
    GotoNotes,
    GotoHelp,
    OpenPalette,
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
}

#[derive(Debug, Default)]
pub struct AppState {
    pub selected_note: Option<usize>,
    pub message: String,
    pub palette_open: bool,
    pub palette_input: String,
}

pub type App = TuiApp<View, Action, AppState, Handler, Overlay>;

pub struct Handler;

impl TuiActionHandler<View, Action, AppState, Overlay> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View, Overlay>,
        state: &mut AppState,
        _runtime: RuntimeContext<'_, Action, Overlay>,
    ) -> Result<ActionOutcome<View, Overlay>, Self::Error> {
        // Actions that mean the same thing everywhere are handled once; the
        // rest are routed to whatever page we're on.
        if let Some(outcome) = global_action(action, state) {
            return Ok(outcome);
        }

        Ok(match ctx.current_view {
            View::Home => home::handle(action, &ctx, state),
            View::Notes => notes::handle(action, &ctx, state),
            View::Help => help::handle(action, &ctx, state),
        })
    }
}

fn global_action(action: Action, state: &mut AppState) -> Option<ActionOutcome<View, Overlay>> {
    let outcome = match action {
        Action::Nav(nav) => match nav {
            // Activate is per-page — let the page decide what "enter" means
            // for the currently focused target.
            NavigationAction::Activate => return None,
            nav => ActionOutcome::effect(nav.to_effect()),
        },
        Action::GotoHome => ActionOutcome::effect(TuiEffect::Navigate(View::Home)),
        Action::GotoNotes => ActionOutcome::effect(TuiEffect::Navigate(View::Notes)),
        Action::GotoHelp => ActionOutcome::effect(TuiEffect::Navigate(View::Help)),
        Action::OpenPalette => {
            state.palette_open = true;
            state.palette_input.clear();
            ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Open(FocusTarget::Overlay(
                Overlay::CommandBar,
            ))))
        }
    };
    Some(outcome)
}

fn page_spec(
    view: &View,
    state: &AppState,
    _focus: Option<&FocusTarget<Overlay>>,
) -> PageSpec<Overlay> {
    match view {
        View::Home => home::page_spec(state),
        View::Notes => notes::page_spec(state),
        View::Help => help::page_spec(state),
    }
}

pub fn build() -> App {
    let mut app = TuiPages::builder(View::Home)
        .page_fn(page_spec)
        .handler(Handler)
        // Vim preset covers the standard navigation: focus movement (j/k/l/h,
        // arrows, tab), activate (enter), leave section (esc), quit (ctrl+c),
        // and the workspace operations (next/prev/close buffer, next/close
        // pane, vertical/horizontal split). Anything the preset doesn't cover
        // is bound explicitly below.
        .vim_defaults()
        .vim_navigation_defaults()
        // App-specific bindings: g h / g n / g ? switch views, : opens the
        // command palette. The preset has no notion of either, so they live
        // here.
        .bind(modes::GENERAL, "g h", Action::GotoHome)
        .bind(modes::GENERAL, "g n", Action::GotoNotes)
        .bind(modes::GENERAL, "g ?", Action::GotoHelp)
        .bind(modes::GENERAL, ":", Action::OpenPalette)
        .command("Go to Home", ["h", "home"], Action::GotoHome)
        .command("Go to Notes", ["n", "notes"], Action::GotoNotes)
        .command("Go to Help", ["?", "help"], Action::GotoHelp)
        .command("Quit", ["q", "quit"], Action::Nav(NavigationAction::Quit))
        .build();

    app.refresh_page(&AppState::default());
    app
}
