mod app;
mod form;
mod notes;
mod search;
mod ui;

use anyhow::Result;
use crossterm::event::Event;
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::canvas;
use tui_pages::prelude::*;

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let _input = canvas::CrosstermInputSession::install()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = app::build();
    let mut state = app::AppState::default();
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| ui::render(frame, &tui, &mut state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        let handled = match (*tui.current_view(), tui.focus.current()) {
            (app::View::Notes, Some(FocusTarget::CanvasField(0))) => handle_text_widget(
                canvas::dispatch_text_area_key(&mut state.notes, key),
                &mut tui,
                &state,
            ),
            (app::View::Search, Some(FocusTarget::CanvasField(0))) => handle_text_widget(
                canvas::dispatch_text_input_key(&mut state.search, key),
                &mut tui,
                &state,
            ),
            _ => false,
        };

        if !handled && tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}

fn handle_text_widget(
    outcome: canvas::CanvasTextWidgetOutcome,
    tui: &mut app::App,
    state: &app::AppState,
) -> bool {
    match outcome {
        canvas::CanvasTextWidgetOutcome::Handled => true,
        canvas::CanvasTextWidgetOutcome::Submitted => true,
        canvas::CanvasTextWidgetOutcome::Focus(intent) => {
            tui.apply_effect(TuiEffect::Focus(intent), state);
            true
        }
        canvas::CanvasTextWidgetOutcome::NotHandled => false,
    }
}
