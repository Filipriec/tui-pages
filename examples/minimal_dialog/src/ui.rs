// Pure ratatui rendering. Reads runtime + app state to decide what to draw.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tui_pages::prelude::*;

use crate::app::{App, AppState};

pub fn render(frame: &mut Frame, tui: &App, state: &AppState) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // item list
            Constraint::Length(3), // "Delete an item" button
            Constraint::Length(3), // "Quit" button
            Constraint::Length(1), // message
            Constraint::Length(1), // hint
        ])
        .split(area);

    let focus = tui.focus.current();

    let items: Vec<ListItem> = state
        .items
        .iter()
        .map(|i| ListItem::new(format!("• {i}")))
        .collect();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Items ({}) ", state.items.len())),
        ),
        rows[0],
    );

    render_button(frame, rows[1], "Delete an item", &focus, 0);
    render_button(frame, rows[2], "Quit", &focus, 1);

    frame.render_widget(
        Paragraph::new(state.message.as_str())
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow)),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new("Tab move focus · Enter select · Ctrl+C quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        rows[4],
    );

    // The dialog is a modal overlay owned by the focus manager. When one is
    // open, draw it on top with the built-in renderer.
    if let Some(data) = dialog::current_dialog(&tui.focus) {
        let active = dialog::active_button(&tui.focus).unwrap_or(0);
        render_dialog(frame, area, data, active, &DialogTheme::default());
    }
}

fn render_button(frame: &mut Frame, area: Rect, label: &str, focus: &Option<FocusTarget>, index: usize) {
    // While a dialog is open, focus is on a ModalItem, so neither page
    // button is highlighted — exactly what we want for a modal.
    let focused = matches!(focus, Some(FocusTarget::Button(i)) if *i == index);
    let style = if focused {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
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
