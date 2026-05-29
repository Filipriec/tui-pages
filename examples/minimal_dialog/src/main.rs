mod app;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::dialog::{self, DialogResult};
use tui_pages::{FocusController, FocusIntent};

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

        // A modal dialog intercepts all input until it is answered. We drive it
        // directly with the runtime's focus manager and the `dialog::*` helpers.
        if dialog::current_dialog(&tui.focus).is_some() {
            match key.code {
                KeyCode::Tab | KeyCode::Right => {
                    tui.focus.apply_focus_intent(FocusIntent::Next)
                }
                KeyCode::BackTab | KeyCode::Left => {
                    tui.focus.apply_focus_intent(FocusIntent::Prev)
                }
                KeyCode::Enter => {
                    if let Some(DialogResult::Selected { purpose, index }) =
                        dialog::selection(&tui.focus)
                    {
                        apply_dialog(purpose, index, state);
                    }
                    tui.focus.apply_focus_intent(FocusIntent::ClearOverlay);
                }
                KeyCode::Esc => {
                    state.message = "Cancelled.".into();
                    tui.focus.apply_focus_intent(FocusIntent::ClearOverlay);
                }
                _ => {}
            }
            continue;
        }

        if tui.handle_key(key, state)?.quit_requested {
            return Ok(());
        }
    }
}

fn apply_dialog(purpose: Option<Purpose>, index: usize, state: &mut app::AppState) {
    match (purpose, index) {
        (Some(Purpose::ConfirmDelete), 0) => {
            let removed = state.items.remove(0);
            state.message = format!("Deleted \"{removed}\".");
        }
        (Some(Purpose::ConfirmDelete), _) => state.message = "Deletion cancelled.".into(),
        _ => {}
    }
}
