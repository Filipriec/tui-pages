//! Demo showcasing tui-pages features with ratatui.
//!
//! This example demonstrates:
//! - Mode-based keybindings (normal, insert, global)
//! - Focus navigation between interactive elements
//! - Command palette with fuzzy matching
//! - Multi-key chord sequences (e.g., "Ctrl+x s")
//! - Buffer/view history with back navigation

use anyhow::Result;
use crossterm::event::MouseEvent;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::{backend::CrosstermBackend, Terminal};

use tui_pages::{
    modes, ActionContext, ActionOutcome, FocusIntent, FocusTarget, KeyChord, PageSpec,
    TuiActionHandler, TuiEffect, TuiPages, TuiPagesOutput, TuiPagesStatus,
};

mod pages;

// ============================================================================
// Application Types
// ============================================================================

/// All views/screens in the demo app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Home,
    Settings,
    Form,
    Info,
}

/// All actions the app can handle.
///
/// Using an enum keeps actions type-safe and exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    // Focus navigation
    FocusNext,
    FocusPrev,
    Activate,
    // View navigation
    GoHome,
    GoSettings,
    GoForm,
    GoInfo,
    GoBack,
    // Commands
    Save,
    Quit,
    // Special
    ToggleCommandPalette,
}

/// App-wide state.
#[derive(Debug, Default)]
pub struct AppState {
    pub saved: bool,
    pub form_name: String,
    pub form_email: String,
    pub command_mode: bool,
    pub message: Option<String>,
}

// ============================================================================
// Action Handler
// ============================================================================

pub(crate) struct Handler;

impl TuiActionHandler<AppView, AppAction, AppState> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: AppAction,
        ctx: ActionContext<AppView>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<AppView>, Self::Error> {
        use TuiEffect::*;

        match action {
            // Focus actions
            AppAction::FocusNext => {
                return Ok(ActionOutcome::effect(Focus(FocusIntent::Next)));
            }
            AppAction::FocusPrev => {
                return Ok(ActionOutcome::effect(Focus(FocusIntent::Prev)));
            }
            AppAction::Activate => {
                // Trigger behavior based on current focus target
                match ctx.focus {
                    Some(FocusTarget::Button(0)) => {
                        if ctx.current_view == AppView::Home {
                            return Ok(ActionOutcome::effect(Navigate(AppView::Settings)));
                        }
                    }
                    Some(FocusTarget::Button(1)) => {
                        if ctx.current_view == AppView::Home {
                            return Ok(ActionOutcome::effect(Navigate(AppView::Form)));
                        }
                    }
                    Some(FocusTarget::Button(2)) => {
                        if ctx.current_view == AppView::Home {
                            return Ok(ActionOutcome::effect(Focus(FocusIntent::Open(
                                FocusTarget::Overlay(tui_pages::focus::OverlayKind::CommandBar),
                            ))));
                        }
                    }
                    _ => {}
                }
                return Ok(ActionOutcome::none());
            }

            // View navigation
            AppAction::GoHome => return Ok(ActionOutcome::effect(Navigate(AppView::Home))),
            AppAction::GoSettings => return Ok(ActionOutcome::effect(Navigate(AppView::Settings))),
            AppAction::GoForm => return Ok(ActionOutcome::effect(Navigate(AppView::Form))),
            AppAction::GoInfo => return Ok(ActionOutcome::effect(Navigate(AppView::Info))),

            // Buffer/history navigation
            AppAction::GoBack => {
                return Ok(ActionOutcome::effect(PreviousBuffer));
            }

            // Commands
            AppAction::Save => {
                state.saved = true;
                state.message = Some("Saved!".to_string());
                return Ok(ActionOutcome::effect(RefreshPage));
            }
            AppAction::Quit => {
                return Ok(ActionOutcome::effect(Quit));
            }

            // Special
            AppAction::ToggleCommandPalette => {
                state.command_mode = !state.command_mode;
                return Ok(ActionOutcome::effect(Focus(FocusIntent::Toggle(
                    FocusTarget::Overlay(tui_pages::focus::OverlayKind::CommandBar),
                ))));
            }
        }
    }

    fn handle_text(
        &mut self,
        chord: KeyChord,
        _ctx: ActionContext<AppView>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<AppView>, Self::Error> {
        // Collect text input for form fields
        if let crossterm::event::KeyCode::Char(c) = chord.code {
            state.form_name.push(c);
        }
        Ok(ActionOutcome::none())
    }
}

// ============================================================================
// Page Specifications
// ============================================================================

fn page_spec(view: &AppView, _state: &AppState, _focus: Option<&FocusTarget>) -> PageSpec {
    match view {
        AppView::Home => PageSpec::new()
            .focus_targets(vec![
                FocusTarget::Button(0),
                FocusTarget::Button(1),
                FocusTarget::Button(2),
            ])
            .modes(vec![modes::GENERAL, modes::GLOBAL]),

        AppView::Settings => PageSpec::new()
            .focus_targets(vec![
                FocusTarget::Button(0),
                FocusTarget::Button(1),
                FocusTarget::Section(0),
            ])
            .modes(vec![modes::GENERAL, modes::GLOBAL]),

        AppView::Form => PageSpec::new()
            .focus_targets(vec![
                FocusTarget::Button(0),
                FocusTarget::Button(1),
            ])
            .modes(vec![modes::GENERAL, modes::NORMAL, modes::GLOBAL])
            .accepts_text_input(true),

        AppView::Info => PageSpec::new()
            .focus_targets(vec![
                FocusTarget::Button(0),
                FocusTarget::Button(1),
            ])
            .modes(vec![modes::GENERAL, modes::GLOBAL]),
    }
}

// ============================================================================
// TUI Setup
// ============================================================================

pub(crate) type Tui = TuiPages<
    AppView,
    AppAction,
    AppState,
    fn(&AppView, &AppState, Option<&FocusTarget>) -> PageSpec,
    Handler,
>;

fn build_tui() -> Tui {
    TuiPages::builder(AppView::Home)
        .pages(page_spec as fn(&AppView, &AppState, Option<&FocusTarget>) -> PageSpec)
        .handler(Handler)
        // General mode bindings
        .bind(modes::GENERAL, "tab", AppAction::FocusNext)
        .bind(modes::GENERAL, "shift+tab", AppAction::FocusPrev)
        .bind(modes::GENERAL, "enter", AppAction::Activate)
        .bind(modes::GENERAL, "space", AppAction::Activate)
        .bind(modes::GENERAL, "h", AppAction::GoHome)
        .bind(modes::GENERAL, "s", AppAction::GoSettings)
        .bind(modes::GENERAL, "f", AppAction::GoForm)
        .bind(modes::GENERAL, "?", AppAction::ToggleCommandPalette)
        // Normal mode (vim-like)
        .bind(modes::NORMAL, "j", AppAction::FocusNext)
        .bind(modes::NORMAL, "k", AppAction::FocusPrev)
        // Global bindings work in any mode
        .bind(modes::GLOBAL, "ctrl+q", AppAction::Quit)
        .bind(modes::GLOBAL, "escape", AppAction::GoBack)
        .bind(modes::GLOBAL, "ctrl+h", AppAction::GoHome)
        // Commands
        .command("Save", ["save", "s"], AppAction::Save)
        .command("Quit", ["quit", "q"], AppAction::Quit)
        .command("Settings", ["settings"], AppAction::GoSettings)
        .command("Form", ["form"], AppAction::GoForm)
        .command("Info", ["info"], AppAction::GoInfo)
        .build()
}

// ============================================================================
// Main Event Loop
// ============================================================================

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(std::io::stderr());
    let mut terminal = Terminal::new(backend)?;

    // Create app state and TUI runtime
    let mut app_state = AppState::default();
    let mut tui = build_tui();

    // Main event loop
    loop {
        // Render
        terminal.draw(|f| {
            let area = f.area();
            pages::render(f, area, &tui, &app_state);
        })?;

        // Handle input
        let event = crossterm::event::read()?;

        match event {
            crossterm::event::Event::Key(key) => {
                let output = tui.handle_key(key, &mut app_state)?;

                if handle_output(output, &mut app_state) {
                    break;
                }
            }
            crossterm::event::Event::Mouse(mouse) => {
                handle_mouse(mouse, &mut app_state, &mut tui);
            }
            crossterm::event::Event::Resize(_, _) => {
                // Ratatui auto-handles resize in draw closure
            }
            crossterm::event::Event::FocusGained
            | crossterm::event::Event::FocusLost
            | crossterm::event::Event::Paste(_) => {}
        }
    }

    // Cleanup
    disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), LeaveAlternateScreen)?;

    println!("\nThanks for using tui-pages demo!");

    Ok(())
}

fn handle_output(output: TuiPagesOutput<AppAction>, state: &mut AppState) -> bool {
    match output.status {
        TuiPagesStatus::ActionHandled => {
            if output.quit_requested {
                return true;
            }
        }
        TuiPagesStatus::CommandIncomplete(hints) => {
            state.message = Some(format!(
                "Possible: {}",
                hints.iter()
                    .map(|h| h.alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        TuiPagesStatus::CommandUnknown => {
            state.message = Some("Unknown command. Press ? for palette.".to_string());
        }
        TuiPagesStatus::Waiting(_hints) => {
            // Visual feedback for chord waiting handled in render
        }
        TuiPagesStatus::CommandEmpty => {
            state.command_mode = false;
        }
        _ => {}
    }

    false
}

fn handle_mouse(_mouse: MouseEvent, state: &mut AppState, tui: &mut Tui) {
    // Mouse handling would go here
    tui.refresh_page(state);
}
