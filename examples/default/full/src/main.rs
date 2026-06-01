mod app;
mod help;
mod home;
mod notes;
mod ui;

use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::{InputHint, TuiPagesStatus};

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let mut tui = app::build();
    let mut state = app::AppState::default();

    run(&mut terminal, &mut tui, &mut state)
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
    tui: &mut app::App,
    state: &mut app::AppState,
) -> Result<()> {
    let mut waiting: Vec<InputHint<app::Action>> = Vec::new();

    loop {
        terminal.draw(|frame| ui::render(frame, tui, state, &waiting))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        // The command palette is plain app state — tui-pages ships no palette,
        // so we drive the text box ourselves and only hand the result to
        // submit_command on Enter.
        if state.palette_open {
            match key.code {
                KeyCode::Enter => {
                    let input = state.palette_input.clone();
                    let quit = tui.submit_command(&input, state)?.quit_requested;
                    close_palette(tui, state);
                    if quit {
                        return Ok(());
                    }
                }
                KeyCode::Esc => close_palette(tui, state),
                KeyCode::Backspace => {
                    state.palette_input.pop();
                }
                KeyCode::Char(c) => state.palette_input.push(c),
                _ => {}
            }
            continue;
        }

        let output = tui.handle_key(key, state)?;
        match output.status {
            TuiPagesStatus::Waiting(hints) => waiting = hints,
            _ => waiting.clear(),
        }

        if output.quit_requested {
            return Ok(());
        }
    }
}

fn close_palette(tui: &mut app::App, state: &mut app::AppState) {
    state.palette_open = false;
    state.palette_input.clear();
    tui.focus.clear_overlay();
}
