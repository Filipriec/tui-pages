# Full Example

A complete TUI application demonstrating navigation, buffers, and panes.

## Features Demonstrated

- Multiple views with different focus targets
- Buffer navigation (back/forward history)
- Pane splitting
- Command palette
- Multiple input modes

## app.rs — Full Setup

```rust
use crossterm::event::KeyEvent;
use tui_pages::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppAction {
    // Focus
    FocusNext,
    FocusPrev,
    Activate,

    // Navigation
    GoToHome,
    GoToSettings,
    GoToEditor,
    NextBuffer,
    PreviousBuffer,

    // Panes
    SplitHorizontal,
    SplitVertical,
    ClosePane,
    NextPane,
    PreviousPane,

    // Commands
    OpenPalette,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AppView {
    Home,
    Settings,
    Editor { file: String },
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub files: Vec<String>,
    pub current_file: Option<usize>,
}

impl App {
    pub fn build() -> TuiPages<AppView, AppAction, AppState, (), ()> {
        TuiPages::builder(AppView::Home)
            .pages(Self::page_spec)
            .handler(Handler)
            .focus_wrap(FocusWrap::Wrap)

            // General mode bindings
            .bind(modes::GENERAL, "tab", AppAction::FocusNext)
            .bind(modes::GENERAL, "S-tab", AppAction::FocusPrev)
            .bind(modes::GENERAL, "enter", AppAction::Activate)
            .bind(modes::GENERAL, "1", AppAction::GoToHome)
            .bind(modes::GENERAL, "2", AppAction::GoToSettings)
            .bind(modes::GENERAL, "3", AppAction::GoToEditor)
            .bind(modes::GENERAL, "C-right", AppAction::NextBuffer)
            .bind(modes::GENERAL, "C-left", AppAction::PreviousBuffer)

            // Pane bindings
            .bind(modes::GENERAL, "C-s h", AppAction::SplitHorizontal)
            .bind(modes::GENERAL, "C-s v", AppAction::SplitVertical)
            .bind(modes::GENERAL, "C-w", AppAction::ClosePane)
            .bind(modes::GENERAL, "C-h", AppAction::PreviousPane)
            .bind(modes::GENERAL, "C-l", AppAction::NextPane)

            // Command palette
            .bind(modes::PALETTE, "escape", AppAction::Quit)
            .command("home", ["h"], AppAction::GoToHome)
            .command("settings", ["s"], AppAction::GoToSettings)
            .command("editor", ["e"], AppAction::GoToEditor)
            .command("quit", ["q"], AppAction::Quit)

            .build()
    }

    fn page_spec(view: &AppView, _state: &AppState, focus: Option<&FocusTarget>) -> PageSpec {
        match view {
            AppView::Home => PageSpec::new()
                .focus_targets(vec![
                    FocusTarget::Button(0),
                    FocusTarget::Button(1),
                    FocusTarget::Button(2),
                ]),

            AppView::Settings => PageSpec::new()
                .focus_targets(vec![
                    FocusTarget::Section { id: 0, item_index: 0, item_count: 5 },
                    FocusTarget::Section { id: 1, item_index: 0, item_count: 3 },
                ]),

            AppView::Editor { .. } => PageSpec::new()
                .focus_targets(vec![FocusTarget::Canvas, FocusTarget::Button(0)])
                .accepts_text_input(true)
                .modes(vec![modes::NORMAL, modes::INSERT, modes::GLOBAL]),
        }
    }
}

struct Handler;

impl TuiActionHandler<AppView, AppAction, AppState, (), ()> for Handler {
    type Error = Infallible;

    fn handle_action(
        &mut self,
        action: AppAction,
        ctx: ActionContext<AppView>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<AppView>, Self::Error> {
        use AppAction::*;
        use TuiEffect::*;

        match action {
            FocusNext => Ok(ActionOutcome::effect(Focus(FocusIntent::Next))),
            FocusPrev => Ok(ActionOutcome::effect(Focus(FocusIntent::Prev))),

            Activate => {
                // Handle button activation based on current view
                Ok(ActionOutcome::effect(RefreshPage))
            }

            GoToHome => Ok(ActionOutcome::effect(Navigate(AppView::Home))),
            GoToSettings => Ok(ActionOutcome::effect(Navigate(AppView::Settings))),
            GoToEditor => Ok(ActionOutcome::effect(Navigate(AppView::Editor {
                file: "untitled.txt".to_string(),
            }))),

            NextBuffer => Ok(ActionOutcome::effect(NextBuffer)),
            PreviousBuffer => Ok(ActionOutcome::effect(PreviousBuffer)),

            SplitHorizontal => Ok(ActionOutcome::effect(SplitPane(PaneSplit::Horizontal))),
            SplitVertical => Ok(ActionOutcome::effect(SplitPane(PaneSplit::Vertical))),
            ClosePane => Ok(ActionOutcome::effect(ClosePane)),
            NextPane => Ok(ActionOutcome::effect(NextPane)),
            PreviousPane => Ok(ActionOutcome::effect(PreviousPane)),

            OpenPalette => {
                // Open command palette overlay
                Ok(ActionOutcome::effect(Open(OverlayFocus::Simple(()))))
            }

            Quit => Ok(ActionOutcome::effect(Quit)),
        }
    }
}
```

## Key Bindings Reference

| Key | Action |
|-----|--------|
| `1`, `2`, `3` | Switch to Home, Settings, Editor |
| `Tab` / `Shift+Tab` | Navigate focus |
| `Ctrl+Left/Right` | Buffer history |
| `Ctrl+s h/v` | Split pane horizontal/vertical |
| `Ctrl+w` | Close pane |
| `Ctrl+h/l` | Switch panes |
| `q` | Quit |

## Adding Custom Modes

```rust
// Create a custom mode
const NOTE_MODE: ModeId = ModeId::borrowed("note");

// Bind keys in that mode
.bind(NOTE_MODE, "C-s", AppAction::Save)
.bind(NOTE_MODE, "C-z", AppAction::Undo)
.bind(NOTE_MODE, "escape", AppAction::ExitNoteMode)

// Page uses the mode
AppView::Editor { .. } => PageSpec::new()
    .modes(vec![NOTE_MODE, modes::GLOBAL])
```

## State Management

```rust
pub struct AppState {
    pub files: Vec<String>,
    pub current_file: Option<usize>,
    pub unsaved_changes: bool,
    pub clipboard: String,
}

impl AppState {
    fn new() -> Self {
        Self {
            files: vec!["main.rs".to_string(), "lib.rs".to_string()],
            current_file: Some(0),
            unsaved_changes: false,
            clipboard: String::new(),
        }
    }
}
```

## Event Loop with Command Palette

```rust
loop {
    terminal.draw(|frame| ui::render(frame, &tui, &state))?;

    if let Event::Key(key) = event::read()? {
        // Check for palette activation
        if key.code == KeyCode::Char(':') {
            let command = read_command_line()?;
            match tui.submit_command(&command, &mut state)? {
                output if output.quit_requested => break,
                _ => {}
            }
        } else {
            match tui.handle_key(key, &mut state)? {
                output if output.quit_requested => break,
                TuiPagesOutput { status: TuiPagesStatus::Waiting(hints), .. } => {
                    // Show key hints
                }
                TuiPagesOutput { status: TuiPagesStatus::TextInput(chord), .. } => {
                    // Handle text input in editor
                }
                _ => {}
            }
        }
    }
}
```
