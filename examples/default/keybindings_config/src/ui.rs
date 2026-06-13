//! Pure rendering. Knows nothing about keybindings — it just draws the current
//! state and shows which keys are bound (passed in from `main`).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use crate::State;

/// The live keys for the actions the footer advertises.
pub struct Keys {
    pub toggle: String,
    pub save: String,
    pub cycle: String,
    pub quit: String,
}

pub fn render(
    frame: &mut Frame,
    state: &State,
    focus: Option<FocusTarget>,
    keys: &Keys,
    buttons: &[String],
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // title
            Constraint::Min(6),    // body
            Constraint::Length(3), // status
            Constraint::Length(6), // keybinding help
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new("tui-pages — keybindings from config.toml")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL)),
        rows[0],
    );

    render_body(frame, rows[1], state, &focus, buttons);

    frame.render_widget(
        Paragraph::new(if state.status.is_empty() {
            "ready"
        } else {
            state.status.as_str()
        })
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().title(" status ").borders(Borders::ALL)),
        rows[2],
    );

    let help = vec![
        Line::from(format!(
            "{}: toggle sidebar   ·   {}: rebind toggle key   ·   {}: save to config.toml",
            keys.toggle, keys.cycle, keys.save
        )),
        Line::from(format!(
            "Tab/↓ next · Shift+Tab/↑ prev · Enter runs the focused button · {} quit",
            keys.quit
        )),
        Line::from("edit config.toml and relaunch to change these bindings"),
    ];
    frame.render_widget(
        Paragraph::new(help)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title(" keybindings ").borders(Borders::ALL)),
        rows[3],
    );
}

fn render_body(
    frame: &mut Frame,
    area: Rect,
    state: &State,
    focus: &Option<FocusTarget>,
    buttons: &[String],
) {
    let columns = if state.sidebar_open {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0)])
            .split(area)
    };

    let main_area = if state.sidebar_open {
        frame.render_widget(
            Paragraph::new("Sidebar\n\n(toggled by a\nconfigured key)")
                .block(Block::default().title(" sidebar ").borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan)),
            columns[0],
        );
        columns[1]
    } else {
        columns[0]
    };

    let mut constraints: Vec<Constraint> = buttons.iter().map(|_| Constraint::Length(3)).collect();
    constraints.push(Constraint::Min(0));
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(main_area);

    for (index, label) in buttons.iter().enumerate() {
        render_button(frame, slots[index], label, focus, index);
    }
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
