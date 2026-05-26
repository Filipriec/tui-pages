mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::{InputHint, TuiPagesStatus};

fn main() -> Result<()> {
    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let mut tui = app::build();
    let mut state = app::AppState::default();
    let mut waiting: Vec<InputHint<app::Action>> = Vec::new();

    let result = run(&mut terminal, &mut tui, &mut state, &mut waiting);

    disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), LeaveAlternateScreen)?;
    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
    tui: &mut app::App,
    state: &mut app::AppState,
    waiting: &mut Vec<InputHint<app::Action>>,
) -> Result<()> {
    loop {
        terminal.draw(|frame| {
            ui::render(
                frame,
                tui,
                state,
                ui::Status {
                    waiting: if waiting.is_empty() { None } else { Some(waiting) },
                },
            )
        })?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        // Palette submission: run the typed command before the Enter key closes the overlay.
        let submitted_quit = if state.palette_open && key.code == KeyCode::Enter {
            let input = state.palette_input.clone();
            tui.submit_command(&input, state)?.quit_requested
        } else {
            false
        };

        let output = tui.handle_key(key, state)?;
        match output.status {
            TuiPagesStatus::Waiting(hints) => *waiting = hints,
            _ => waiting.clear(),
        }

        if submitted_quit || output.quit_requested {
            return Ok(());
        }
    }
}
