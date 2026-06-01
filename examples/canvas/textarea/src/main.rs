//! Everything that is *not* the `tui-pages` contract: the textarea state, the
//! side-effect functions the handler calls, the ratatui rendering, and the
//! event loop.
//!
//! The textarea is a single top-level focus stop. `j`/`k` step past it to the
//! buttons; Enter *enters* it. Once entered it is modal: NORMAL hands keys to
//! the runtime (move the cursor between lines, `i` to insert), INSERT hands raw
//! keys to the textarea's full editor (newlines, joins, wrapping), and `Esc`
//! steps back out (INSERT -> NORMAL -> top-level stop).

mod app;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use tui_pages::canvas;
use tui_pages::prelude::*;

/// Application state: a single multi-line textarea plus whether it is currently
/// entered (selected for inner navigation) or just a top-level focus stop.
pub struct State {
    pub body: canvas::TextAreaState<canvas::TextAreaProvider>,
    pub in_textarea: bool,
}

impl Default for State {
    fn default() -> Self {
        let mut body = canvas::TextAreaState::from_text(
            "Enter selects this textarea.\nThen i edits, Esc leaves edit mode.\nj/k move between lines once you are inside.\nWhile it is just a stop, j/k jump straight to the buttons.",
        );
        body.use_wrap();
        Self {
            body,
            in_textarea: false,
        }
    }
}

/// The "Clear" button's effect: empty the textarea (back in NORMAL, un-entered).
/// The handler in `app.rs` calls this; the runtime knows nothing about it.
pub fn clear_textarea(state: &mut State) {
    let mut body = canvas::TextAreaState::from_text("");
    body.use_wrap();
    state.body = body;
}

/// The two buttons below the textarea.
const BUTTON_LABELS: [&str; 2] = ["Clear", "Quit"];

/// Clear a lone `Shift` modifier on character keys so the textarea's
/// modifier-free insert path accepts capitals and shifted symbols. Ctrl/Alt
/// combos (real shortcuts) are left untouched.
fn normalize_shift(mut key: crossterm::event::KeyEvent) -> crossterm::event::KeyEvent {
    if matches!(key.code, KeyCode::Char(_)) && key.modifiers == KeyModifiers::SHIFT {
        key.modifiers = KeyModifiers::NONE;
    }
    key
}

fn mode_label(mode: canvas::AppMode) -> &'static str {
    match mode {
        canvas::AppMode::Edit => "INSERT",
        canvas::AppMode::ReadOnly => "NORMAL",
        canvas::AppMode::Highlight => "VISUAL",
        canvas::AppMode::Command => "COMMAND",
        canvas::AppMode::General => "GENERAL",
    }
}

/// Draw the textarea and the two buttons. `on_textarea` is true when the
/// textarea holds focus (as a stop); `entered` is true once it has been
/// selected for inner navigation; `mode` is its editor mode.
fn render(
    frame: &mut Frame,
    focused_button: Option<usize>,
    on_textarea: bool,
    entered: bool,
    mode: canvas::AppMode,
    state: &mut State,
) {
    let rows = Layout::vertical([
        Constraint::Min(3),    // the textarea
        Constraint::Length(3), // the two buttons
    ])
    .split(frame.area());

    // The block title shows the mode once entered, and a hint otherwise; the
    // border distinguishes "entered" (green) from "selected stop" (cyan).
    let (title, border) = if entered {
        (
            format!(" Body — {} ", mode_label(mode)),
            Style::default().fg(Color::Green),
        )
    } else if on_textarea {
        (
            String::from(" Body (Enter to edit) "),
            Style::default().fg(Color::Cyan),
        )
    } else {
        (String::from(" Body "), Style::default().fg(Color::DarkGray))
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    frame.render_stateful_widget(
        canvas::TextArea::default().block(Block::default()),
        inner,
        &mut state.body,
    );
    // Show the cursor only while inside the textarea.
    if entered {
        frame.set_cursor_position(state.body.cursor(inner, None));
    }

    // The two buttons.
    let cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);
    for (index, label) in BUTTON_LABELS.iter().enumerate() {
        let focused = focused_button == Some(index);
        let style = if focused {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(
            Paragraph::new(*label)
                .alignment(Alignment::Center)
                .style(style)
                .block(Block::default().borders(Borders::ALL)),
            cols[index],
        );
    }
}

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = app::build();
    let mut state = State::default();
    tui.refresh_page(&state);

    loop {
        let current = tui.focus.current();
        let on_textarea = matches!(&current, Some(FocusTarget::CanvasField(_)));
        // Leaving the textarea (focus moved to a button) always un-enters it, so
        // coming back lands on the top-level stop rather than back inside.
        if !on_textarea {
            state.in_textarea = false;
        }
        let focused_button = match &current {
            Some(FocusTarget::Button(index)) => Some(*index),
            _ => None,
        };
        let entered = on_textarea && state.in_textarea;
        let mode = state.body.mode();
        let editing = entered && mode == canvas::AppMode::Edit;

        terminal.draw(|frame| render(frame, focused_button, on_textarea, entered, mode, &mut state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        // INSERT mode (entered): the textarea owns the keys. Esc returns to
        // NORMAL; Ctrl+C still quits. Everything else flows through the runtime.
        if editing {
            match (key.code, key.modifiers) {
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                (KeyCode::Esc, _) => {
                    let _ = state.body.exit_edit_mode();
                }
                _ => {
                    let _ = state.body.input(normalize_shift(key));
                }
            }
            continue;
        }

        if tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}
