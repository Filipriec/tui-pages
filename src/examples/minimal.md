# Minimal Example

A minimal TUI application with two buttons and Tab navigation.

## Project Structure

```
examples/minimal/
├── Cargo.toml
└── src/
    ├── main.rs    # Entry point and event loop
    ├── app.rs     # TuiPages setup
    └── ui.rs      # Ratatui rendering
```

## app.rs — Runtime Setup

```rust
use crossterm::event::KeyEvent;
use tui_pages::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppAction {
    FocusNext,
    FocusPrev,
    Activate,  // "Click" current button
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppView {
    Home,
}

pub struct AppState {
    pub message: String,
}

impl App {
    pub fn build() -> TuiPages<AppView, AppAction, AppState> {
        TuiPages::builder(AppView::Home)
            .pages(Self::page_spec)
            .handler(Handler)
            .bind(modes::GENERAL, "tab", AppAction::FocusNext)
            .bind(modes::GENERAL, "S-tab", AppAction::FocusPrev)
            .bind(modes::GENERAL, "enter", AppAction::Activate)
            .bind(modes::GENERAL, "q", AppAction::Quit)
            .command("quit", ["q"], AppAction::Quit)
            .build()
    }

    fn page_spec(view: &AppView, _state: &AppState, _focus: Option<&FocusTarget>) -> PageSpec {
        match view {
            AppView::Home => PageSpec::new()
                .focus_targets(vec![FocusTarget::Button(0), FocusTarget::Button(1)]),
        }
    }
}

struct Handler;

impl TuiActionHandler<AppView, AppAction, AppState> for Handler {
    type Error = Infallible;

    fn handle_action(
        &mut self,
        action: AppAction,
        _ctx: ActionContext<AppView>,
        state: &mut AppState,
    ) -> Result<ActionOutcome<AppView>, Self::Error> {
        match action {
            AppAction::FocusNext => Ok(ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next))),

            AppAction::FocusPrev => Ok(ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev))),

            AppAction::Activate => {
                state.message = format!("Activated button {:?}", self.focus.current());
                Ok(ActionOutcome::none())
            }

            AppAction::Quit => Ok(ActionOutcome::effect(TuiEffect::Quit)),
        }
    }
}
```

## main.rs — Event Loop

```rust
mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    // Build TUI runtime
    let mut tui = app::build();
    let mut state = app::AppState {
        message: "Ready".to_string(),
    };

    // Main event loop
    loop {
        // Render
        terminal.draw(|frame| ui::render(frame, &tui, &state))?;

        // Handle input
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            if key.kind != crossterm::event::KeyEventKind::Press {
                continue;
            }

            match tui.handle_key(key, &mut state)? {
                output if output.quit_requested => break,
                TuiPagesOutput { status: TuiPagesStatus::Waiting(hints), .. } => {
                    // Could show hints in status bar
                    println!("Waiting for: {:?}", hints);
                }
                _ => {}
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), LeaveAlternateScreen)?;
    Ok(())
}
```

## ui.rs — Rendering

```rust
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn render(
    frame: &mut Frame,
    tui: &TuiPages<app::AppView, app::AppAction, app::AppState>,
    state: &app::AppState,
) {
    let area = frame.size();

    // Create button areas
    let button_area = Rect::new(area.x + 2, area.y + 2, area.width - 4, 5);
    let button_width = (button_area.width - 2) / 2;

    // Draw buttons
    for i in 0..2 {
        let button_rect = Rect::new(
            button_area.x + (i as u16) * (button_width + 2),
            button_area.y,
            button_width,
            3,
        );

        let focused = matches!(
            tui.focus.current(),
            Some(FocusTarget::Button(idx)) if idx == i
        );

        let style = if focused {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        };

        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Button {}", i + 1))
                .style(style),
            button_rect,
        );
    }

    // Draw status
    let status = Paragraph::new(state.message.as_str())
        .block(Block::default().borders(Borders::TOP).title("Status"));

    frame.render_widget(status, Rect::new(2, area.height - 3, area.width - 4, 3));
}
```

## Building and Running

```bash
cd examples/minimal
cargo run
```

## Expected Behavior

- Press **Tab** to cycle focus between buttons
- Press **Shift+Tab** to cycle backwards
- Press **Enter** to "activate" the focused button
- Press **q** to quit
