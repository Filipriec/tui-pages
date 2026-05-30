use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use super::logic;
use crate::app::Overlay;

pub fn render(frame: &mut Frame, area: Rect, focus: Option<FocusTarget<Overlay>>) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from("Welcome to the tui-pages full demo."),
            Line::from(""),
            Line::from("Try the multi-key chord: press g, then h / n / ?"),
            Line::from("Or open the palette with `:` and type `q`, `n`, `home`, `quit`."),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" Home ")),
        rows[0],
    );

    crate::ui::render_button(frame, rows[1], logic::BUTTONS[0], focus.clone(), 0);
    crate::ui::render_button(frame, rows[2], logic::BUTTONS[1], focus, 1);
}
