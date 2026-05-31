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
    let rows = Layout::vertical([Constraint::Length(3), Constraint::Length(3), Constraint::Min(0)])
        .split(area);
    let block = Block::default().borders(Borders::ALL).title(logic::TITLE);
    let input_area = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    frame.render_stateful_widget(
        canvas::TextInput::default().block(Block::default()),
        input_area,
        &mut state.search,
    );

    if matches!(focus, Some(FocusTarget::CanvasField(0))) {
        frame.set_cursor_position(state.search.cursor(input_area, None));
    }

    crate::ui::render_button(
        frame,
        rows[1],
        "Back to form page",
        matches!(focus, Some(FocusTarget::Button(0))),
    );
}
