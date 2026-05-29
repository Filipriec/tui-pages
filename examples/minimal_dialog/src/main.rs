mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::Event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::prelude::*;

use app::Purpose;

fn main() -> Result<()> {
    enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let mut tui = app::build();
    let mut state = app::AppState::default();

    let result = run(&mut terminal, &mut tui, &mut state);

    disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), LeaveAlternateScreen)?;
    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stderr>>,
    tui: &mut app::App,
    state: &mut app::AppState,
) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, tui, state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        // A modal dialog intercepts input until it is answered. `dialog::handle_key`
        // applies the conventional bindings (Tab/arrows move, Enter selects, Esc
        // dismisses) and closes the dialog for us — we only act on the result.
        match dialog::handle_key(&mut tui.focus, key) {
            DialogKey::Ignored => {
                if tui.handle_key(key, state)?.quit_requested {
                    return Ok(());
                }
            }
            DialogKey::Consumed => {}
            DialogKey::Resolved(result) => apply_dialog(result, state),
        }
    }
}

fn apply_dialog(result: DialogResult<Purpose>, state: &mut app::AppState) {
    match result {
        DialogResult::Selected {
            purpose: Some(Purpose::ConfirmDelete),
            index: 0,
        } => {
            let removed = state.items.remove(0);
            state.message = format!("Deleted \"{removed}\".");
        }
        DialogResult::Selected { .. } => state.message = "Deletion cancelled.".into(),
        DialogResult::Dismissed => state.message = "Cancelled.".into(),
    }
}
