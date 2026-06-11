//! Everything that is *not* the `tui-pages` contract

mod app;

use anyhow::Result;
use crossterm::event::Event;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};
use tui_pages::canvas;
use tui_pages::prelude::*;

/// The form's backing data: two fields the editor reads and writes.
#[derive(Debug)]
pub struct Contact {
    pub values: Vec<String>,
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            values: vec!["Ada".to_string(), "ada@example.test".to_string()],
        }
    }
}

impl canvas::DataProvider for Contact {
    fn field_count(&self) -> usize {
        self.values.len()
    }

    fn field_name(&self, index: usize) -> &str {
        match index {
            0 => "Name",
            1 => "Email",
            _ => "",
        }
    }

    fn field_value(&self, index: usize) -> &str {
        &self.values[index]
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.values[index] = value;
    }
}

/// Application state: just the form editor in this example.
pub struct State {
    pub editor: canvas::FormEditor<Contact>,
}

impl canvas::CanvasWidgetState for State {
    fn canvas_form_editor_ref(&self, id: usize) -> Option<&dyn canvas::CanvasFormEditorHost> {
        match id {
            0 => Some(&self.editor),
            _ => None,
        }
    }

    fn canvas_form_editor(
        &mut self,
        id: usize,
    ) -> Option<&mut dyn canvas::CanvasFormEditorHost> {
        match id {
            0 => Some(&mut self.editor),
            _ => None,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            editor: canvas::FormEditor::new(Contact::default()),
        }
    }
}

/// The "Clear" button's effect: wipe both fields by resetting the editor. The
/// handler in `app.rs` calls this; the runtime knows nothing about it.
pub fn clear_form(state: &mut State) {
    state.editor = canvas::FormEditor::new(Contact {
        values: vec![String::new(), String::new()],
    });
}

/// The two buttons below the form.
const BUTTON_LABELS: [&str; 2] = ["Clear", "Quit"];

/// Draw the form and the two buttons. `focused_button` is the index of the
/// focused button, or `None` while focus is inside the canvas.
fn render(frame: &mut Frame, focused_button: Option<usize>, state: &State) {
    let area = frame.area();
    let rows = Layout::vertical([
        Constraint::Length(6), // the two form fields
        Constraint::Length(3), // the two buttons
        Constraint::Min(0),
    ])
    .split(area);

    canvas::render_canvas_with_suggestions_default(frame, area, rows[0], &state.editor);

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
        let focused_button = match tui.focus.current() {
            Some(FocusTarget::Button(index)) => Some(index),
            _ => None,
        };

        terminal.draw(|frame| render(frame, focused_button, &state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };
        if tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}
