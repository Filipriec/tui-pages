use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
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
    state: &mut AppState,
    focus: Option<&FocusTarget<Overlay>>,
) {
    let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);

    let on_textarea = matches!(focus, Some(FocusTarget::CanvasField(_)));
    let entered = state.entered;
    let mode = state.body.mode();

    // The title shows the mode once entered, a hint while it is a selected stop;
    // the border distinguishes entered (green) from selected (cyan).
    let (title, border) = if entered {
        (
            format!(" Body — {} ", logic::mode_label(mode)),
            Style::default().fg(Color::Green),
        )
    } else if on_textarea {
        (
            String::from(" Body (Enter to edit) "),
            Style::default().fg(Color::Cyan),
        )
    } else {
        (String::from(" Body "), Style::default().fg(Color::DarkGray))
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border);
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);
    frame.render_stateful_widget(
        canvas::TextArea::default().block(Block::default()),
        inner,
        &mut state.body,
    );
    if entered {
        frame.set_cursor_position(state.body.cursor(inner, None));
    }

    let cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(rows[1]);
    crate::ui::render_button(
        frame,
        cols[0],
        logic::BUTTONS[0],
        matches!(focus, Some(FocusTarget::Button(0))),
    );
    crate::ui::render_button(
        frame,
        cols[1],
        logic::BUTTONS[1],
        matches!(focus, Some(FocusTarget::Button(1))),
    );
}
