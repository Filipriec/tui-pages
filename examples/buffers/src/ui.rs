// Pure ratatui rendering. Reads the `BufferState` the runtime maintains and
// draws (a) the buffer history strip and (b) the pane layout. It never mutates
// anything — all buffer changes happen through effects in app.rs.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_pages::{BufferState, PaneSplit};

use crate::app::View;

pub fn render(frame: &mut Frame, buffer: &BufferState<View>) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // buffer history strip
            Constraint::Min(0),    // panes
            Constraint::Length(6), // help
        ])
        .split(frame.area());

    render_buffer_strip(frame, rows[0], buffer);
    render_panes(frame, rows[1], buffer);
    render_help(frame, rows[2], buffer);
}

/// The open buffers (the navigation history), active one highlighted.
fn render_buffer_strip(frame: &mut Frame, area: Rect, buffer: &BufferState<View>) {
    let mut spans = vec![Span::styled("buffers: ", Style::default().fg(Color::DarkGray))];
    for (i, view) in buffer.history.iter().enumerate() {
        let active = i == buffer.active_index;
        let label = format!(" {} ", view.name());
        spans.push(if active {
            Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(label, Style::default().fg(Color::Gray))
        });
        spans.push(Span::raw(" "));
    }

    frame.render_widget(
        Paragraph::new(Line::from(spans)).block(
            Block::default()
                .title(" Buffer history ")
                .borders(Borders::ALL),
        ),
        area,
    );
}

/// The workspace: one pane, or several after a split.
fn render_panes(frame: &mut Frame, area: Rect, buffer: &BufferState<View>) {
    let panes = &buffer.workspace.panes;
    let active = buffer.workspace.active_pane;

    let direction = match buffer.workspace.split {
        Some(PaneSplit::Horizontal) => Direction::Vertical,
        // Vertical split / no split lay out left-to-right.
        _ => Direction::Horizontal,
    };

    let constraints: Vec<Constraint> =
        panes.iter().map(|_| Constraint::Ratio(1, panes.len() as u32)).collect();
    let cells = Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area);

    for (i, pane) in panes.iter().enumerate() {
        let is_active = i == active;
        let style = if is_active {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let body = format!(
            "\n{}\n\npane #{}  (id {})",
            pane.view.name(),
            i,
            pane.pane_id.0
        );
        frame.render_widget(
            Paragraph::new(body)
                .alignment(Alignment::Center)
                .style(style)
                .block(
                    Block::default()
                        .title(if is_active { " ● active " } else { " ○ " })
                        .borders(Borders::ALL)
                        .border_style(style),
                ),
            cells[i],
        );
    }
}

fn render_help(frame: &mut Frame, area: Rect, buffer: &BufferState<View>) {
    let split = match buffer.split_direction() {
        Some(PaneSplit::Vertical) => "vertical",
        Some(PaneSplit::Horizontal) => "horizontal",
        None => "none",
    };
    let status = format!(
        "buffers: {}   panes: {}   split: {}",
        buffer.history.len(),
        buffer.workspace.panes.len(),
        split,
    );

    let lines = vec![
        Line::from(Span::styled(status, Style::default().fg(Color::Cyan))),
        Line::from(
            "1/2/3 open buffer  ·  Tab / Shift+Tab cycle buffers  ·  w close buffer",
        ),
        Line::from(
            "v split │  ·  s split ─  ·  o / p next/prev pane  ·  x close pane  ·  Ctrl+C quit",
        ),
    ];

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL)),
        area,
    );
}
