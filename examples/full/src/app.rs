// Everything in this file uses `tui-pages`. The UI layer (ui.rs) only reads
// runtime state to draw; it owns no coordination logic.
//
// Capabilities exercised:
//   • Views + navigation            TuiEffect::Navigate
//   • Focus cycling + activation     FocusIntent::Next/Prev, FocusTarget::Button
//   • Sections with items            EnterSection / LeaveSection, SectionItem
//   • Multi-key chord sequences      `g h`, `g n`, `g ?`
//   • Buffer history                 NextBuffer / PreviousBuffer / CloseBuffer
//   • Pane splits                    SplitPane / NextPane / ClosePane
//   • Command palette                built in app space from the runtime's
//                                    public command resolver — see main.rs.
//                                    The crate ships no palette; you compose one.

use tui_pages::{
    modes, ActionContext, ActionOutcome, FocusIntent, FocusTarget, OverlayKind, PageFocusBuilder,
    PageSpec, PaneSplit, TuiActionHandler, TuiEffect, TuiPages,
};

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
    MoveUp,
    MoveDown,
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

pub type App = TuiPages<
    View,
    Action,
    AppState,
    fn(&View, &AppState, Option<&FocusTarget>) -> PageSpec,
    Handler,
>;

pub struct Handler;

impl TuiActionHandler<View, Action, AppState> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::FocusNext => top_level_focus(ctx.focus, FocusIntent::Next),
            Action::FocusPrev => top_level_focus(ctx.focus, FocusIntent::Prev),
            Action::MoveUp => inside_section(ctx.focus, FocusIntent::Prev),
            Action::MoveDown => inside_section(ctx.focus, FocusIntent::Next),
            Action::Select => select(ctx, state),
            Action::Escape => escape(ctx),

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
                    FocusTarget::Overlay(OverlayKind::CommandBar),
                )))
            }

            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

fn top_level_focus(focus: Option<FocusTarget>, intent: FocusIntent) -> ActionOutcome<View> {
    if matches!(focus, Some(FocusTarget::SectionItem { .. })) {
        return ActionOutcome::effects([
            TuiEffect::Focus(FocusIntent::LeaveSection),
            TuiEffect::Focus(intent),
        ]);
    }
    ActionOutcome::effect(TuiEffect::Focus(intent))
}

fn inside_section(focus: Option<FocusTarget>, intent: FocusIntent) -> ActionOutcome<View> {
    if matches!(focus, Some(FocusTarget::SectionItem { .. })) {
        ActionOutcome::effect(TuiEffect::Focus(intent))
    } else {
        ActionOutcome::none()
    }
}

fn select(ctx: ActionContext<View>, state: &mut AppState) -> ActionOutcome<View> {
    match (ctx.current_view, ctx.focus) {
        (View::Notes, Some(FocusTarget::Section(NOTES_SECTION))) => {
            ActionOutcome::effect(TuiEffect::Focus(FocusIntent::EnterSection {
                item_count: NOTES.len(),
            }))
        }
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
        _ => ActionOutcome::none(),
    }
}

fn escape(ctx: ActionContext<View>) -> ActionOutcome<View> {
    if matches!(ctx.focus, Some(FocusTarget::SectionItem { .. })) {
        ActionOutcome::effect(TuiEffect::Focus(FocusIntent::LeaveSection))
    } else {
        ActionOutcome::none()
    }
}

fn page_spec(view: &View, _state: &AppState, _focus: Option<&FocusTarget>) -> PageSpec {
    let mut focus = PageFocusBuilder::new();
    match view {
        View::Home => focus = focus.button(0).button(1),
        View::Notes => focus = focus.section(NOTES_SECTION).button(0),
        View::Help => {}
    }

    PageSpec::new()
        .focus_targets(focus.build())
        .modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn build() -> App {
    let mut app = TuiPages::builder(View::Home)
        .pages(page_spec as fn(&View, &AppState, Option<&FocusTarget>) -> PageSpec)
        .handler(Handler)
        // Focus + activation
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "j", Action::MoveDown)
        .bind(modes::GENERAL, "k", Action::MoveUp)
        .bind(modes::GENERAL, "down", Action::MoveDown)
        .bind(modes::GENERAL, "up", Action::MoveUp)
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
