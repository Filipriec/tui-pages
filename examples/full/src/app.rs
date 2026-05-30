// Everything in this file uses `tui-pages`. The UI layer (ui.rs) only reads
// runtime state to draw; it owns no coordination logic.
//
// Capabilities exercised:
//   • Views + navigation            TuiEffect::Navigate
//   • Focus cycling + activation     FocusIntent::Next/Prev/Activate, Button
//   • Sections with items            section_with_items + Activate / LeaveSection
//                                    — the runtime enters/leaves and moves
//                                    within sections; the app never inspects
//                                    focus to route a keypress.
//   • Multi-key chord sequences      `g h`, `g n`, `g ?`
//   • Buffer history                 NextBuffer / PreviousBuffer / CloseBuffer
//   • Pane splits                    SplitPane / NextPane / ClosePane
//   • Command palette                built in app space from the runtime's
//                                    public command resolver — see main.rs.
//                                    The crate ships no palette; you compose one.

use tui_pages::prelude::*;
use tui_pages::PaneSplit;

/// The overlays this TUI has. The crate provides no overlay names — this is the
/// app's own type, threaded through the runtime as the `O` parameter so focus
/// targets are compiler-checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Overlay {
    CommandBar,
}

pub const NOTES_SECTION: usize = 0;
pub const NOTES: [&str; 4] = [
    "Buy milk",
    "Write blog post",
    "Read tui-pages docs",
    "Refactor input pipeline",
];

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
    Select,
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
}

#[derive(Debug, Default)]
pub struct AppState {
    pub selected_note: Option<usize>,
    pub message: String,
    // The command palette is ordinary app state. The crate does not own it.
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
        Ok(match action {
            // Movement is now the runtime's job. `Next`/`Prev` move within an
            // entered section and step out to the next top-level target at its
            // edge — so the same intent drives Tab and j/k, and the handler no
            // longer inspects focus to decide where a keypress should go.
            Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
            Action::Select => select(ctx, state),
            // `LeaveSection` is a no-op when no section is entered, so Esc needs
            // no focus check either.
            Action::Escape => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::LeaveSection)),

            Action::GotoHome => ActionOutcome::effect(TuiEffect::Navigate(View::Home)),
            Action::GotoNotes => ActionOutcome::effect(TuiEffect::Navigate(View::Notes)),
            Action::GotoHelp => ActionOutcome::effect(TuiEffect::Navigate(View::Help)),

            Action::NextBuffer => ActionOutcome::effect(TuiEffect::NextBuffer),
            Action::PrevBuffer => ActionOutcome::effect(TuiEffect::PreviousBuffer),
            Action::CloseBuffer => ActionOutcome::effect(TuiEffect::CloseBuffer),

            Action::SplitVertical => {
                ActionOutcome::effect(TuiEffect::SplitPane(PaneSplit::Vertical))
            }
            Action::SplitHorizontal => {
                ActionOutcome::effect(TuiEffect::SplitPane(PaneSplit::Horizontal))
            }
            Action::NextPane => ActionOutcome::effect(TuiEffect::NextPane),
            Action::ClosePane => ActionOutcome::effect(TuiEffect::ClosePane),

            // Open is just app state plus a focus effect. Typing, submitting,
            // and closing are handled by the app's own loop in main.rs.
            Action::OpenPalette => {
                state.palette_open = true;
                state.palette_input.clear();
                ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Open(
                    FocusTarget::Overlay(Overlay::CommandBar),
                )))
            }

            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

// `select` now carries only genuine application meaning — which view a button
// goes to, what selecting a note does. The "enter the focused section" case is
// gone: pressing Enter on a section falls through to `FocusIntent::Activate`,
// and the runtime enters it using the item count declared in `page_spec`.
fn select(ctx: ActionContext<View, Overlay>, state: &mut AppState) -> ActionOutcome<View, Overlay> {
    match (ctx.current_view, ctx.focus) {
        (
            View::Notes,
            Some(FocusTarget::SectionItem {
                section: NOTES_SECTION,
                item,
            }),
        ) => {
            state.selected_note = Some(item);
            state.message = format!("Selected note: {}", NOTES[item]);
            ActionOutcome::effect(TuiEffect::RefreshPage)
        }
        (View::Notes, Some(FocusTarget::Button(0))) => {
            ActionOutcome::effect(TuiEffect::Navigate(View::Home))
        }
        (View::Home, Some(FocusTarget::Button(0))) => {
            ActionOutcome::effect(TuiEffect::Navigate(View::Notes))
        }
        (View::Home, Some(FocusTarget::Button(1))) => {
            ActionOutcome::effect(TuiEffect::Navigate(View::Help))
        }
        // Enter on anything else (e.g. the Notes section header) → activate it.
        _ => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Activate)),
    }
}

fn page_spec(view: &View, _state: &AppState, _focus: Option<&FocusTarget<Overlay>>) -> PageSpec<Overlay> {
    let mut focus = PageFocusBuilder::new();
    match view {
        View::Home => focus = focus.button(0).button(1),
        // The item count travels with the section, so the runtime can enter it
        // on Activate without the handler ever passing `NOTES.len()`.
        View::Notes => focus = focus.section_with_items(NOTES_SECTION, NOTES.len()).button(0),
        View::Help => {}
    }

    PageSpec::new()
        .focus(focus)
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn build() -> App {
    let mut app = TuiPages::builder(View::Home)
        .page_fn(page_spec)
        .handler(Handler)
        // Focus + activation. Tab and j/k share one intent: the focus manager
        // moves within a section when one is entered and steps to the next
        // top-level target at its edge.
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "j", Action::FocusNext)
        .bind(modes::GENERAL, "k", Action::FocusPrev)
        .bind(modes::GENERAL, "down", Action::FocusNext)
        .bind(modes::GENERAL, "up", Action::FocusPrev)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GENERAL, "esc", Action::Escape)
        // Multi-key chords: press `g`, then h / n / ?
        .bind(modes::GENERAL, "g h", Action::GotoHome)
        .bind(modes::GENERAL, "g n", Action::GotoNotes)
        .bind(modes::GENERAL, "g ?", Action::GotoHelp)
        // Buffers
        .bind(modes::GENERAL, "]", Action::NextBuffer)
        .bind(modes::GENERAL, "[", Action::PrevBuffer)
        .bind(modes::GENERAL, "x", Action::CloseBuffer)
        // Panes
        .bind(modes::GENERAL, "ctrl+s", Action::SplitVertical)
        .bind(modes::GENERAL, "ctrl+d", Action::SplitHorizontal)
        .bind(modes::GENERAL, "ctrl+n", Action::NextPane)
        .bind(modes::GENERAL, "ctrl+w", Action::ClosePane)
        // Command line (`:` opens it; runtime handles typing / Enter / Esc)
        .bind(modes::GENERAL, ":", Action::OpenPalette)
        // Quit works everywhere, including while the command line is open
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        // Commands typed into the command line
        .command("Go to Home", ["h", "home"], Action::GotoHome)
        .command("Go to Notes", ["n", "notes"], Action::GotoNotes)
        .command("Go to Help", ["?", "help"], Action::GotoHelp)
        .command("Quit", ["q", "quit"], Action::Quit)
        .build();

    app.refresh_page(&AppState::default());
    app
}
