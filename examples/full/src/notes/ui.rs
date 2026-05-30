use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use super::logic;
use crate::app::{AppState, Overlay};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    focus: Option<FocusTarget<Overlay>>,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let section_focused = matches!(
        focus,
        Some(FocusTarget::Section(logic::SECTION))
            | Some(FocusTarget::SectionItem {
                section: logic::SECTION,
                ..
            })
    );
    let active_item = match focus {
        Some(FocusTarget::SectionItem {
            section: logic::SECTION,
            item,
        }) => Some(item),
        _ => state.selected_note,
    };

    let items = logic::NOTES
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mark = if Some(i) == state.selected_note {
                "[x]"
            } else {
                "[ ]"
            };
            ListItem::new(format!("{mark} {n}"))
        })
        .collect::<Vec<_>>();

    let mut list_state = ListState::default();
    list_state.select(active_item);

    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Notes ")
                    .borders(Borders::ALL)
                    .border_style(if section_focused {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .highlight_symbol("> ")
            .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        cols[0],
        &mut list_state,
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(cols[1]);

    crate::ui::render_button(frame, rows[0], "Back to Home", focus.clone(), 0);

    let detail = match state.selected_note {
        Some(i) => format!("Selected:\n\n  {}", logic::NOTES[i]),
        None => "Press Enter on the list, then j/k, then Enter to pick a note.".to_string(),
    };
    frame.render_widget(
        Paragraph::new(detail).block(Block::default().borders(Borders::ALL).title(" Detail ")),
        rows[1],
    );
}
