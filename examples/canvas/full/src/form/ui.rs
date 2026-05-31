use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders},
    Frame,
};
use tui_pages::canvas;
use tui_pages::FocusTarget;

use crate::app::AppState;

pub fn render(frame: &mut Frame, area: Rect, state: &AppState, focus: Option<&FocusTarget>) {
    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).split(area);
    let block = Block::default().borders(Borders::ALL).title(" FormEditor ");
    let form_area = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    canvas::render_canvas_with_suggestions_default(frame, frame.area(), form_area, &state.form);

    crate::ui::render_button(
        frame,
        rows[1],
        "Open textarea page",
        matches!(focus, Some(FocusTarget::Button(0))),
    );
}
