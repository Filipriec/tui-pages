use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use crate::{AppState, AppView, DemoTui, OPTION_SECTION, OPTIONS};

pub fn render(frame: &mut Frame, tui: &DemoTui, state: &AppState) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .split(area);

    render_header(frame, layout[0], tui);
    render_body(frame, layout[1], tui, state);
    render_footer(frame, layout[2], tui, state);
}

fn render_header(frame: &mut Frame, area: Rect, tui: &DemoTui) {
    let active = tui.current_view();
    let history = tui
        .buffer
        .history
        .iter()
        .enumerate()
        .map(|(index, view)| {
            if index == tui.buffer.active_index {
                format!("[{:?}]", view)
            } else {
                format!("{:?}", view)
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    let pane_line = tui
        .buffer
        .panes()
        .iter()
        .enumerate()
        .map(|(index, pane)| {
            if index == tui.buffer.active_pane_index() {
                format!("*{}:{:?}", pane.pane_id.0, pane.view)
            } else {
                format!(" {}:{:?}", pane.pane_id.0, pane.view)
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    let lines = vec![
        Line::from(vec![
            Span::styled("tui-pages buffer demo", Style::default().bold()),
            Span::raw(format!("  current: {:?}", active)),
        ]),
        Line::from(format!("buffers: {history}")),
        Line::from(format!("panes: {pane_line}")),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::White)),
        area,
    );
}

fn render_body(frame: &mut Frame, area: Rect, tui: &DemoTui, state: &AppState) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    render_page(frame, columns[0], tui.current_view());
    render_options(frame, columns[1], tui, state);
    render_buttons(frame, columns[2], tui);
}

fn render_page(frame: &mut Frame, area: Rect, view: &AppView) {
    let text = match view {
        AppView::Home => vec![
            Line::from("This is the Home page."),
            Line::from("Press 2 or 3 to open another page."),
            Line::from("Opening a page pushes it into buffer history."),
        ],
        AppView::Options => vec![
            Line::from("This is the Options page."),
            Line::from("Enter the options section, move with j/k, select with Enter."),
        ],
        AppView::Details => vec![
            Line::from("This is the Details page."),
            Line::from("Use [ and ] to move through buffers."),
            Line::from("Use v to split, p to select pane, x to close pane."),
        ],
    };

    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Page ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .alignment(Alignment::Left),
        area,
    );
}

fn render_options(frame: &mut Frame, area: Rect, tui: &DemoTui, state: &AppState) {
    let focus = tui.focus.current();
    let section_focused = matches!(
        focus,
        Some(FocusTarget::Section(OPTION_SECTION))
            | Some(FocusTarget::SectionItem {
                section: OPTION_SECTION,
                ..
            })
    );
    let active_item = match focus {
        Some(FocusTarget::SectionItem {
            section: OPTION_SECTION,
            item,
        }) => Some(item),
        _ => Some(state.selected_option),
    };

    let items = OPTIONS
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let marker = if index == state.selected_option {
                "[x] "
            } else {
                "[ ] "
            };
            ListItem::new(format!("{marker}{option}"))
        })
        .collect::<Vec<_>>();

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(active_item);

    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Options ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(if section_focused {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .highlight_symbol("> ")
            .highlight_style(Style::default().fg(Color::Green)),
        area,
        &mut list_state,
    );
}

fn render_buttons(frame: &mut Frame, area: Rect, tui: &DemoTui) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    render_button(frame, rows[0], tui, 0, "Home");
    render_button(frame, rows[1], tui, 1, "Options");
    render_button(frame, rows[2], tui, 2, "Details");
}

fn render_button(frame: &mut Frame, area: Rect, tui: &DemoTui, index: usize, label: &str) {
    let focused = matches!(tui.focus.current(), Some(FocusTarget::Button(current)) if current == index);
    let style = if focused {
        Style::default().fg(Color::Green).bold()
    } else {
        Style::default().fg(Color::White)
    };

    frame.render_widget(
        Paragraph::new(label)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(style),
            )
            .style(style)
            .alignment(Alignment::Center),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, tui: &DemoTui, state: &AppState) {
    let focus = tui
        .focus
        .current()
        .map(|focus| format!("{focus:?}"))
        .unwrap_or_else(|| "None".to_string());

    let lines = vec![
        Line::from("Tab/Shift+Tab focus | Enter select | j/k options | 1/2/3 pages | [/] buffers | v split | p pane | x close | Ctrl+C quit"),
        Line::from(format!("focus: {focus}")),
        Line::from(state.message.as_str()),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            )
            .style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
