use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_pages::{FocusTarget, InputHint, PaneSplit};

use crate::app::{self, App, AppState, Overlay, View};
use crate::{help, home, notes};

pub fn render(frame: &mut Frame, tui: &App, state: &AppState, waiting: &[InputHint<app::Action>]) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    render_tabs(frame, rows[0], tui);
    render_workspace(frame, rows[1], tui);
    render_body(frame, rows[2], tui, state);
    render_status(frame, rows[3], tui, state, waiting);

    if state.palette_open {
        render_palette(frame, area, &state.palette_input);
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, tui: &App) {
    let current = tui.current_view();
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
            tab("Home", *current == View::Home),
            Span::raw("  "),
            tab("Notes", *current == View::Notes),
            Span::raw("  "),
            tab("Help", *current == View::Help),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" views ")),
        area,
    );
}

fn render_workspace(frame: &mut Frame, area: Rect, tui: &App) {
    let buffers = tui
        .buffer
        .history
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == tui.buffer.active_index {
                format!("*{v:?}")
            } else {
                format!(" {v:?}")
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    let panes = tui
        .buffer
        .panes()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == tui.buffer.active_pane_index() {
                format!("*{}:{:?}", p.pane_id.0, p.view)
            } else {
                format!(" {}:{:?}", p.pane_id.0, p.view)
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("buffers: {buffers}")),
            Line::from(format!("panes:   {panes}")),
        ])
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title(" workspace ")),
        area,
    );
}

fn render_body(frame: &mut Frame, area: Rect, tui: &App, state: &AppState) {
    let panes = tui.buffer.panes();
    let active_idx = tui.buffer.active_pane_index();

    let regions: Vec<Rect> = if panes.len() <= 1 {
        vec![area]
    } else {
        let direction = match tui.buffer.split_direction() {
            Some(PaneSplit::Horizontal) => Direction::Vertical,
            _ => Direction::Horizontal,
        };
        let count = panes.len() as u32;
        let constraints: Vec<Constraint> =
            (0..panes.len()).map(|_| Constraint::Ratio(1, count)).collect();
        Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(area)
            .to_vec()
    };

    for (i, pane) in panes.iter().enumerate() {
        let is_active = i == active_idx;
        let focus = if is_active { tui.focus.current() } else { None };
        render_pane(frame, regions[i], pane.view, state, focus, is_active);
    }
}

fn render_pane(
    frame: &mut Frame,
    area: Rect,
    view: View,
    state: &AppState,
    focus: Option<FocusTarget<Overlay>>,
    is_active: bool,
) {
    let border = Style::default().fg(if is_active { Color::Green } else { Color::DarkGray });
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(format!(" {} ", if is_active { "● active" } else { "○ inactive" }));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    match view {
        View::Home => home::ui::render(frame, inner, focus),
        View::Notes => notes::ui::render(frame, inner, state, focus),
        View::Help => help::ui::render(frame, inner),
    }
}

pub fn render_button(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    focus: Option<FocusTarget<Overlay>>,
    index: usize,
) {
    let focused = matches!(focus, Some(FocusTarget::Button(i)) if i == index);
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

fn render_status(
    frame: &mut Frame,
    area: Rect,
    tui: &App,
    state: &AppState,
    waiting: &[InputHint<app::Action>],
) {
    let focus_label = tui
        .focus
        .current()
        .map(|f| format!("{f:?}"))
        .unwrap_or_else(|| "None".into());

    let waiting = if waiting.is_empty() {
        None
    } else {
        let preview = waiting
            .iter()
            .take(4)
            .map(|h| h.key.display_string())
            .collect::<Vec<_>>()
            .join(" / ");
        Some(format!("…waiting: {preview}"))
    };

    let mut lines = vec![Line::from(format!("focus: {focus_label}"))];
    if let Some(w) = waiting {
        lines.push(Line::from(Span::styled(
            w,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
    } else if !state.message.is_empty() {
        lines.push(Line::from(state.message.as_str()));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(Color::Gray))
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
