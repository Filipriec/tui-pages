// Shared chrome: the tab bar, the status bar, the command palette overlay, and
// the button helper every page reuses. Page bodies are drawn by the per-page
// `ui::render` functions.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_pages::prelude::*;

use crate::app::{App, AppState, Overlay, View};
use crate::{editor, form, help};

pub fn render(frame: &mut Frame, tui: &App, state: &mut AppState) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let current = *tui.current_view();
    render_tabs(frame, rows[0], current);

    let focus = tui.focus.current();
    let focus = focus.as_ref();
    match current {
        View::Form => form::ui::render(frame, rows[1], state, focus),
        View::Editor => editor::ui::render(frame, rows[1], state, focus),
        View::Help => help::ui::render(frame, rows[1]),
    }

    render_status(frame, rows[2], state, focus);

    if state.palette_open {
        render_palette(frame, area, &state.palette_input);
    }

    // The login dialog is a modal overlay owned by the focus manager. When one
    // is open, draw it on top with the built-in renderer.
    if let Some(data) = dialog::current_dialog(&tui.focus) {
        let active = dialog::active_button(&tui.focus).unwrap_or(0);
        render_dialog(frame, area, data, active, &DialogTheme::default());
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, current: View) {
    let tab = |name: &str, active: bool| {
        if active {
            Span::styled(
                format!(" [{name}] "),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(format!("  {name}  "), Style::default().fg(Color::DarkGray))
        }
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            tab("Form", current == View::Form),
            Span::raw("  "),
            tab("Editor", current == View::Editor),
            Span::raw("  "),
            tab("Help", current == View::Help),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" canvas full ")),
        area,
    );
}

fn render_status(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    focus: Option<&FocusTarget<Overlay>>,
) {
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("focus: {focus:?}")),
            Line::from(state.message.as_str()),
        ])
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(" status ")),
        area,
    );
}

fn render_palette(frame: &mut Frame, area: Rect, input: &str) {
    let width = area.width.saturating_sub(10).min(60);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + area.height / 3;
    let rect = Rect::new(x, y, width, 3);

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(format!(":{input}"))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" command palette — Enter to run, Esc to close ")
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        rect,
    );
}

/// A single bordered button, highlighted when focused.
pub fn render_button(frame: &mut Frame, area: Rect, label: &str, focused: bool) {
    let style = if focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(style)
            .block(Block::default().borders(Borders::ALL).border_style(style)),
        area,
    );
}
