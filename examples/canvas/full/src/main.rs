//! A canvas demo built on the same project structure as `examples/full`: a thin
//! `main.rs` event loop, an `app.rs` wiring layer, a shared `ui.rs`, and one
//! folder per page (`form/`, `editor/`, `help/`).
//!
//! * The **Form** page hosts a `FormEditor` (two fields) plus two buttons.
//! * The **Editor** page hosts a multi-line `TextArea` plus two buttons.
//! * The **Help** page is a static keybindings cheat-sheet.
//! * `:` opens a command palette (plain app state we drive here, handed to
//!   `submit_command` on Enter).
//!
//! All canvas key handling lives in the runtime: the form editor and textarea
//! widgets are registered on the builder in `app.rs`, so `main.rs` only draws
//! and forwards keys.

mod app;
mod editor;
mod form;
mod help;
mod ui;

use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::prelude::*;

use app::Purpose;

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = app::build();
    let mut state = app::AppState::default();
    tui.refresh_page(&state);

    loop {
        // Keep the state's view in sync with the runtime so `CanvasWidgetState`
        // routes canvas keys to the widget on the visible page.
        state.view = *tui.current_view();

        terminal.draw(|frame| ui::render(frame, &tui, &mut state))?;

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
                    let quit = tui.submit_command(&input, &mut state)?.quit_requested;
                    close_palette(&mut tui, &mut state);
                    if quit {
                        break;
                    }
                }
                KeyCode::Esc => close_palette(&mut tui, &mut state),
                KeyCode::Backspace => {
                    state.palette_input.pop();
                }
                KeyCode::Char(c) => state.palette_input.push(c),
                _ => {}
            }
            continue;
        }

        // When the Role field's suggestion list is open, make it selectable with
        // intuitive keys: Up/Down move the highlight, Enter/Tab accept it. (The
        // editor's own defaults only bind Ctrl+p / Ctrl+n / Ctrl+y for this.)
        if state.view == app::View::Form && state.form.is_suggestions_active() {
            match key.code {
                KeyCode::Up => {
                    state.form.suggestions_prev();
                    continue;
                }
                KeyCode::Down => {
                    state.form.suggestions_next();
                    continue;
                }
                KeyCode::Enter | KeyCode::Tab => {
                    state.form.apply_suggestion();
                    continue;
                }
                _ => {}
            }
        }

        // While a dialog is open, let the vim keys move between its buttons too —
        // `dialog::handle_key` only knows Tab/arrows, so h/k and l/j would
        // otherwise be swallowed with no effect.
        if dialog::current_dialog(&tui.focus).is_some() {
            match key.code {
                KeyCode::Char('h') | KeyCode::Char('k') => {
                    tui.focus.apply_focus_intent(FocusIntent::Prev);
                    continue;
                }
                KeyCode::Char('l') | KeyCode::Char('j') => {
                    tui.focus.apply_focus_intent(FocusIntent::Next);
                    continue;
                }
                _ => {}
            }
        }

        // A modal dialog intercepts input until it is answered. `dialog::handle_key`
        // applies the conventional bindings (Tab/arrows move, Enter selects, Esc
        // dismisses) and closes the dialog for us — we only act on the result.
        match dialog::handle_key(&mut tui.focus, key) {
            DialogKey::Ignored => {
                if tui.handle_key(key, &mut state)?.quit_requested {
                    break;
                }
            }
            DialogKey::Consumed => {}
            DialogKey::Resolved(result) => apply_dialog(result, &mut state),
        }
    }

    Ok(())
}

fn apply_dialog(result: DialogResult<Purpose>, state: &mut app::AppState) {
    match result {
        DialogResult::Selected {
            purpose: Some(Purpose::PostLogin),
            index: 0,
        } => {
            let contact = state.form.data_provider();
            let name = contact.values.first().cloned().unwrap_or_default();
            state.message = format!("Posted login for \"{name}\".");
        }
        DialogResult::Selected { .. } => state.message = "Login cancelled.".into(),
        DialogResult::Dismissed => state.message = "Login dismissed.".into(),
    }
}

fn close_palette(tui: &mut app::App, state: &mut app::AppState) {
    state.palette_open = false;
    state.palette_input.clear();
    tui.focus.clear_overlay();
}
