use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};
use tui_pages::canvas;
use tui_pages::FocusTarget;

use super::logic;
use crate::app::{AppState, Overlay};

pub fn render(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    focus: Option<&FocusTarget<Overlay>>,
) {
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);

    let block = Block::default().borders(Borders::ALL).title(" Contact ");
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    canvas::render_canvas_with_suggestions_default(frame, frame.area(), inner, &state.form);

    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(rows[1]);
    for (index, label) in logic::BUTTONS.iter().enumerate() {
        crate::ui::render_button(
            frame,
            cols[index],
            label,
            matches!(focus, Some(FocusTarget::Button(i)) if *i == index),
        );
    }
}
