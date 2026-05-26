// Everything here uses `tui-pages`. The UI layer (ui.rs) does not.
//
// Features exercised:
//   • Views with Navigate                  (TuiEffect::Navigate)
//   • Tab / Shift+Tab focus + Enter select (FocusIntent::Next/Prev, Button)
//   • Section + j/k items                  (Section / SectionItem, EnterSection/LeaveSection)
//   • Multi-key chord sequences            (`g h`, `g n`, `g ?` → see status bar hints)
//   • Buffer history                       (NextBuffer / PreviousBuffer / CloseBuffer)
//   • Pane split + cycle                   (SplitPane / NextPane / ClosePane)
//   • Command palette `:` overlay          (accepts_text_input + COMMAND mode + submit_command)
//   • Modes: GENERAL / GLOBAL / COMMAND

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
    ClosePalette,
    SubmitPalette,
    PaletteBackspace,

    Quit,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub selected_note: Option<usize>,
    pub message: String,
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
            Action::Escape => escape(ctx, state),

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

            Action::OpenPalette => {
                state.palette_open = true;
                state.palette_input.clear();
                ActionOutcome::effects([
                    TuiEffect::Focus(FocusIntent::Open(FocusTarget::Overlay(
                        OverlayKind::CommandBar,
                    ))),
                    TuiEffect::RefreshPage,
                ])
            }
            Action::ClosePalette => close_palette(state),
            Action::SubmitPalette => {
                // Actual submission happens in main.rs (needs &mut tui).
                // Here we just close the overlay; main runs submit_command first.
                close_palette(state)
            }
            Action::PaletteBackspace => {
                state.palette_input.pop();
                ActionOutcome::none()
            }

            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }

    fn handle_text(
        &mut self,
        chord: tui_pages::KeyChord,
        _ctx: ActionContext<View>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        if state.palette_open {
            if let crossterm::event::KeyCode::Char(c) = chord.code {
                state.palette_input.push(c);
            }
        }
        Ok(ActionOutcome::none())
    }
}

fn close_palette(state: &mut AppState) -> ActionOutcome<View> {
    state.palette_open = false;
    state.palette_input.clear();
    ActionOutcome::effects([
        TuiEffect::Focus(FocusIntent::ClearOverlay),
        TuiEffect::RefreshPage,
    ])
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
    match ctx.focus {
        Some(FocusTarget::Section(NOTES_SECTION)) => {
            ActionOutcome::effect(TuiEffect::Focus(FocusIntent::EnterSection {
                item_count: NOTES.len(),
            }))
        }
        Some(FocusTarget::SectionItem {
            section: NOTES_SECTION,
            item,
        }) => {
            state.selected_note = Some(item);
            state.message = format!("Selected note: {}", NOTES[item]);
            ActionOutcome::effect(TuiEffect::RefreshPage)
        }
        Some(FocusTarget::Button(0)) => ActionOutcome::effect(TuiEffect::Navigate(View::Notes)),
        Some(FocusTarget::Button(1)) => ActionOutcome::effect(TuiEffect::Navigate(View::Help)),
        _ => ActionOutcome::none(),
    }
}

fn escape(ctx: ActionContext<View>, _state: &mut AppState) -> ActionOutcome<View> {
    if matches!(ctx.focus, Some(FocusTarget::SectionItem { .. })) {
        ActionOutcome::effect(TuiEffect::Focus(FocusIntent::LeaveSection))
    } else {
        ActionOutcome::none()
    }
}

fn page_spec(view: &View, state: &AppState, _focus: Option<&FocusTarget>) -> PageSpec {
    if state.palette_open {
        // Text-input mode: plain chars bypass GENERAL bindings and reach handle_text.
        return PageSpec::new()
            .modes(vec![modes::COMMAND, modes::GLOBAL])
            .accepts_text_input(true);
    }

    let mut focus = PageFocusBuilder::new();
    match view {
        View::Home => {
            focus = focus.button(0).button(1);
        }
        View::Notes => {
            focus = focus.section(NOTES_SECTION).button(0);
        }
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
        // Focus and selection
        .bind(modes::GENERAL, "tab", Action::FocusNext)
        .bind(modes::GENERAL, "shift+tab", Action::FocusPrev)
        .bind(modes::GENERAL, "j", Action::MoveDown)
        .bind(modes::GENERAL, "k", Action::MoveUp)
        .bind(modes::GENERAL, "down", Action::MoveDown)
        .bind(modes::GENERAL, "up", Action::MoveUp)
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GENERAL, "esc", Action::Escape)
        // Multi-key chords (try `g`, then `h` / `n` / `?`)
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
        // Palette
        .bind(modes::GENERAL, ":", Action::OpenPalette)
        .bind(modes::COMMAND, "esc", Action::ClosePalette)
        .bind(modes::COMMAND, "enter", Action::SubmitPalette)
        .bind(modes::COMMAND, "backspace", Action::PaletteBackspace)
        // Global
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        // Command palette entries (typed after `:`)
        .command("Go to Home", ["h", "home"], Action::GotoHome)
        .command("Go to Notes", ["n", "notes"], Action::GotoNotes)
        .command("Go to Help", ["?", "help"], Action::GotoHelp)
        .command("Quit", ["q", "quit"], Action::Quit)
        .build();

    app.refresh_page(&AppState::default());
    app
}
