// The wiring layer. Shared types, the key bindings, and two routers that fan
// out to the per-page modules. No page logic lives here — each page owns that in
// its own folder (home/, notes/, help/).

use tui_pages::prelude::*;
use tui_pages::PaneSplit;

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

#[derive(Debug, Clone, Copy)]
pub enum Action {
    FocusNext,
    FocusPrev,
    Escape,

    GotoHome,
    GotoNotes,
    GotoHelp,

    NextBuffer,
    PrevBuffer,
    CloseBuffer,

    SplitVertical,
    SplitHorizontal,
    NextPane,
    ClosePane,

    OpenPalette,
    Quit,

    Select,
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
        Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
        Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
        Action::Escape => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::LeaveSection)),

        Action::GotoHome => ActionOutcome::effect(TuiEffect::Navigate(View::Home)),
        Action::GotoNotes => ActionOutcome::effect(TuiEffect::Navigate(View::Notes)),
        Action::GotoHelp => ActionOutcome::effect(TuiEffect::Navigate(View::Help)),

        Action::NextBuffer => ActionOutcome::effect(TuiEffect::NextBuffer),
        Action::PrevBuffer => ActionOutcome::effect(TuiEffect::PreviousBuffer),
        Action::CloseBuffer => ActionOutcome::effect(TuiEffect::CloseBuffer),

        Action::SplitVertical => ActionOutcome::effect(TuiEffect::SplitPane(PaneSplit::Vertical)),
        Action::SplitHorizontal => {
            ActionOutcome::effect(TuiEffect::SplitPane(PaneSplit::Horizontal))
        }
        Action::NextPane => ActionOutcome::effect(TuiEffect::NextPane),
        Action::ClosePane => ActionOutcome::effect(TuiEffect::ClosePane),

        Action::OpenPalette => {
            state.palette_open = true;
            state.palette_input.clear();
            ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Open(FocusTarget::Overlay(
                Overlay::CommandBar,
            ))))
        }

        Action::Quit => ActionOutcome::effect(TuiEffect::Quit),

        // Select depends on where you are — let the page decide.
        Action::Select => return None,
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
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "j", Action::FocusNext)
        .bind(modes::GENERAL, "k", Action::FocusPrev)
        .bind(modes::GENERAL, "down", Action::FocusNext)
        .bind(modes::GENERAL, "up", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GENERAL, "esc", Action::Escape)
        .bind(modes::GENERAL, "g h", Action::GotoHome)
        .bind(modes::GENERAL, "g n", Action::GotoNotes)
        .bind(modes::GENERAL, "g ?", Action::GotoHelp)
        .bind(modes::GENERAL, "]", Action::NextBuffer)
        .bind(modes::GENERAL, "[", Action::PrevBuffer)
        .bind(modes::GENERAL, "x", Action::CloseBuffer)
        .bind(modes::GENERAL, "ctrl+s", Action::SplitVertical)
        .bind(modes::GENERAL, "ctrl+d", Action::SplitHorizontal)
        .bind(modes::GENERAL, "ctrl+n", Action::NextPane)
        .bind(modes::GENERAL, "ctrl+w", Action::ClosePane)
        .bind(modes::GENERAL, ":", Action::OpenPalette)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .command("Go to Home", ["h", "home"], Action::GotoHome)
        .command("Go to Notes", ["n", "notes"], Action::GotoNotes)
        .command("Go to Help", ["?", "help"], Action::GotoHelp)
        .command("Quit", ["q", "quit"], Action::Quit)
        .build();

    app.refresh_page(&AppState::default());
    app
}
