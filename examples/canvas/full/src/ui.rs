use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use crate::app::{App, AppState, View};
use crate::{form, notes, search};

pub fn render(frame: &mut Frame, tui: &App, state: &mut AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    render_tabs(frame, rows[0], *tui.current_view());
    let focus = tui.focus.current();
    let focus = focus.as_ref();
    match tui.current_view() {
        View::Form => form::ui::render(frame, rows[1], state, focus),
        View::Notes => notes::ui::render(frame, rows[1], state, focus),
        View::Search => search::ui::render(frame, rows[1], state, focus),
    }
    render_status(frame, rows[2], state, focus);
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
            tab("Notes", current == View::Notes),
            Span::raw("  "),
            tab("Search", current == View::Search),
        ]))
        .block(Block::default().borders(Borders::ALL).title(" canvas full ")),
        area,
    );
}

fn render_status(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    focus: Option<&FocusTarget>,
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

pub fn render_button(frame: &mut Frame, area: Rect, label: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    frame.render_widget(
        Paragraph::new(label)
            .style(style)
            .block(Block::default().borders(Borders::ALL).border_style(style)),
        area,
    );
}
