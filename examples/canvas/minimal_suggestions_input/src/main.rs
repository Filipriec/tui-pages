//! Everything that is *not* the `tui-pages` contract: the text-input state, the
//! word list that drives the inline suggestion, the side-effect function the
//! handler calls, the ratatui rendering, and the event loop.
//!
//! The input is a single top-level focus stop. `j`/`k` step past it to the
//! buttons; Enter *enters* it. Once entered it is modal: NORMAL moves the cursor
//! and `i` starts insert, INSERT types into the field. While typing, the runtime
//! asks `canvas_textinput_suggestion_suffix` for a completion and renders it as
//! ghost text; `Tab` accepts the suffix. `Esc` steps back out.

mod app;

use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use tui_pages::canvas;
use tui_pages::prelude::*;

/// The candidate words completed inline as you type.
const WORDS: [&str; 8] = [
    "apple",
    "apricot",
    "banana",
    "blueberry",
    "cherry",
    "cranberry",
    "grape",
    "grapefruit",
];

/// Application state: a single-line input plus whether it is currently entered
/// (selected for inner editing) or just a top-level focus stop.
pub struct State {
    pub input: canvas::TextInputState<canvas::TextInputProvider>,
    pub entered: bool,
}

impl canvas::CanvasWidgetState for State {
    fn canvas_textinput_ref(
        &self,
        focus_index: usize,
    ) -> Option<&dyn canvas::CanvasTextInputHost> {
        match focus_index {
            0 => Some(&self.input),
            _ => None,
        }
    }

    fn canvas_textinput(
        &mut self,
        focus_index: usize,
    ) -> Option<&mut dyn canvas::CanvasTextInputHost> {
        match focus_index {
            0 => Some(&mut self.input),
            _ => None,
        }
    }

    fn canvas_textinput_entered(&mut self, focus_index: usize) -> Option<&mut bool> {
        match focus_index {
            0 => Some(&mut self.entered),
            _ => None,
        }
    }

    fn canvas_textinput_entered_ref(&self, focus_index: usize) -> Option<&bool> {
        match focus_index {
            0 => Some(&self.entered),
            _ => None,
        }
    }

    /// The inline suggestion: the first word that starts with what's typed,
    /// minus the part already typed. Returning `None` clears the ghost text.
    fn canvas_textinput_suggestion_suffix(
        &mut self,
        focus_index: usize,
        text: &str,
    ) -> Option<String> {
        if focus_index != 0 || text.is_empty() {
            return None;
        }
        WORDS
            .iter()
            .find(|word| word.len() > text.len() && word.starts_with(text))
            .map(|word| word[text.len()..].to_string())
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            input: canvas::TextInputState::from_text(""),
            entered: false,
        }
    }
}

/// The "Clear" button's effect: empty the input (back in NORMAL, un-entered).
pub fn clear_input(state: &mut State) {
    state.input = canvas::TextInputState::from_text("");
    state.entered = false;
}

/// The two buttons below the input.
const BUTTON_LABELS: [&str; 2] = ["Clear", "Quit"];

fn mode_label(mode: canvas::AppMode) -> &'static str {
    match mode {
        canvas::AppMode::Ins => "INSERT",
        canvas::AppMode::Nor => "NORMAL",
        canvas::AppMode::Sel => "VISUAL",
        canvas::AppMode::Command => "COMMAND",
        canvas::AppMode::General => "GENERAL",
    }
}

fn render(
    frame: &mut Frame,
    focused_button: Option<usize>,
    on_input: bool,
    entered: bool,
    mode: canvas::AppMode,
    state: &mut State,
) {
    let rows = Layout::vertical([
        Constraint::Length(3), // the input
        Constraint::Length(3), // the two buttons
        Constraint::Length(2), // hint
    ])
    .split(frame.area());

    // Title shows the mode once entered, a hint otherwise; the border
    // distinguishes "entered" (green) from "selected stop" (cyan).
    let (title, border) = if on_input {
        (
            format!(" Fruit — {} (Tab completes, Esc leaves) ", mode_label(mode)),
            Style::default().fg(Color::Green),
        )
    } else {
        (String::from(" Fruit "), Style::default().fg(Color::DarkGray))
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    frame.render_stateful_widget(
        canvas::TextInput::default().block(Block::default()),
        inner,
        &mut state.input,
    );
    if entered {
        frame.set_cursor_position(state.input.cursor(inner, None));
    }

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

    frame.render_widget(
        Paragraph::new("Type a fruit (try \"ap\" or \"gr\") · Tab accepts · Esc/Tab to buttons · Ctrl+C quits")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        rows[2],
    );
}

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = app::build();
    let mut state = State::default();
    tui.refresh_page(&state);

    loop {
        let current = tui.focus.current();
        let on_input = matches!(&current, Some(FocusTarget::CanvasField(_)));

        // Make the input behave like an ordinary text box: the moment it holds
        // focus it is in edit mode, so typing inserts immediately. Without this
        // the field starts in NORMAL and the first letters are read as vim-style
        // commands (e.g. `a`/`i` enter edit instead of being typed), which also
        // means no suggestion is computed and Tab just exits to the buttons.
        if on_input {
            state.entered = true;
            if state.input.mode() != canvas::AppMode::Ins {
                state.input.enter_edit_mode();
            }
        }

        let focused_button = match &current {
            Some(FocusTarget::Button(index)) => Some(*index),
            _ => None,
        };
        let mode = state.input.mode();

        terminal
            .draw(|frame| render(frame, focused_button, on_input, state.entered, mode, &mut state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };

        // Esc leaves the input for the buttons instead of just dropping to
        // NORMAL (which the auto-edit above would immediately undo).
        if on_input && key.code == KeyCode::Esc {
            let _ = state.input.exit_edit_mode();
            state.entered = false;
            tui.focus.apply_focus_intent(FocusIntent::Next);
            continue;
        }

        // The rest — INSERT typing, Tab-completion, the suggestion-suffix
        // refresh, and Tab/arrows exiting to the buttons — is handled by the
        // canvas_textinput_widget hook.
        if tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}
