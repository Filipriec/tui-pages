// Pure ratatui rendering. Reads `View` and `FocusTarget` only to know what to draw.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use crate::app::View;

pub fn render(frame: &mut Frame, view: View, focus: Option<FocusTarget>) {
    let (body, primary) = match view {
        View::Home => ("Welcome to the Home page.", "Go to About"),
        View::About => ("This is the About page.", "Back to Home"),
    };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(frame.area());

    render_tabs(frame, rows[0], view);

    frame.render_widget(
        Paragraph::new(body).alignment(Alignment::Center).block(
            Block::default()
                .title(format!(" {} ", view_name(view)))
                .borders(Borders::ALL),
        ),
        rows[1],
    );

    render_button(frame, rows[2], primary, &focus, 0);
    render_button(frame, rows[3], "Quit", &focus, 1);

    frame.render_widget(
        Paragraph::new("Tab / Shift+Tab to move focus  ·  Enter to select  ·  Ctrl+C to quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray)),
        rows[4],
    );
}

fn view_name(view: View) -> &'static str {
    match view {
        View::Home => "Home",
        View::About => "About",
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, view: View) {
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
            tab("Home", matches!(view, View::Home)),
            Span::raw("  "),
            tab("About", matches!(view, View::About)),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn render_button(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    focus: &Option<FocusTarget>,
    index: usize,
) {
    let focused = matches!(focus, Some(FocusTarget::Button(i)) if *i == index);
    let style = if focused {
        Style::default()
            .fg(Color::Green)
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
