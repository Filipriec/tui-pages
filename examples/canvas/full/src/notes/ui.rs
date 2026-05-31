use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};
use tui_pages::canvas;
use tui_pages::FocusTarget;

use super::logic;
use crate::app::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &mut AppState, focus: Option<&FocusTarget>) {
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);
    let block = Block::default().borders(Borders::ALL).title(logic::TITLE);
    let input_area = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    frame.render_stateful_widget(
        canvas::TextArea::default().block(Block::default()),
        input_area,
        &mut state.notes,
    );

    if matches!(focus, Some(FocusTarget::CanvasField(0))) {
        frame.set_cursor_position(state.notes.cursor(input_area, None));
    }

    crate::ui::render_button(
        frame,
        rows[1],
        "Open text input page",
        matches!(focus, Some(FocusTarget::Button(0))),
    );
}
