//! Ratatui renderer for [`DialogData`].

use crate::dialog::DialogData;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

/// Colors used by [`render_dialog`]. `Default` uses terminal-neutral colors so
/// the dialog looks reasonable on any theme; override fields to match your app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogTheme {
    pub background: Color,
    pub border: Color,
    pub border_active: Color,
    pub title: Color,
    pub text: Color,
    pub button: Color,
    pub button_active: Color,
}

impl Default for DialogTheme {
    fn default() -> Self {
        Self {
            background: Color::Reset,
            border: Color::DarkGray,
            border_active: Color::Cyan,
            title: Color::Cyan,
            text: Color::Reset,
            button: Color::Gray,
            button_active: Color::Cyan,
        }
    }
}

/// Draw a centered modal dialog into `area`.
///
/// `active_button` is the index of the highlighted button — read it from the
/// focus manager (see [`crate::dialog::active_button`]). The dialog is centered
/// within `area`, so pass the full frame area for a true modal.
pub fn render_dialog<D>(
    f: &mut Frame,
    area: Rect,
    data: &DialogData<D>,
    active_button: usize,
    theme: &DialogTheme,
) {
    let message_height = data.message.lines().count().max(1) as u16;
    let button_row_height = if data.buttons.is_empty() { 0 } else { 3 };
    // borders (2) + inner vertical margin (2) + message + buttons.
    let total_height = (message_height + button_row_height + 4).min(area.height.max(3));

    let width = (area.width * 60 / 100).clamp(20, area.width);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(total_height)) / 2;
    let dialog_area = Rect::new(x, y, width, total_height);

    f.render_widget(Clear, dialog_area);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_active))
            .title(format!(" {} ", data.title))
            .title_style(
                Style::default()
                    .fg(theme.title)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(theme.background)),
        dialog_area,
    );

    let inner = dialog_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });

    if data.is_loading {
        f.render_widget(
            Paragraph::new(data.message.as_str())
                .style(
                    Style::default()
                        .fg(theme.text)
                        .add_modifier(Modifier::ITALIC),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let mut constraints = vec![Constraint::Min(message_height.max(1))];
    if button_row_height > 0 {
        constraints.push(Constraint::Length(button_row_height));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    f.render_widget(
        Paragraph::new(Text::from(data.message.as_str()))
            .style(Style::default().fg(theme.text))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        chunks[0],
    );

    if data.buttons.is_empty() || chunks.len() < 2 {
        return;
    }

    let count = data.buttons.len();
    let button_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Ratio(1, count as u32); count])
        .horizontal_margin(1)
        .split(chunks[1]);

    for (i, label) in data.buttons.iter().enumerate() {
        let active = i == active_button;
        let (text_style, border_style) = if active {
            (
                Style::default()
                    .fg(theme.button_active)
                    .add_modifier(Modifier::BOLD),
                Style::default().fg(theme.border_active),
            )
        } else {
            (
                Style::default().fg(theme.button),
                Style::default().fg(theme.border),
            )
        };
        f.render_widget(
            Paragraph::new(label.as_str())
                .alignment(Alignment::Center)
                .style(text_style)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style),
                ),
            button_chunks[i],
        );
    }
}
