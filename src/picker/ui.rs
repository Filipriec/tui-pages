//! Ratatui renderer for [`PickerData`].

use crate::canvas::{render_suggestions_dropdown, DefaultCanvasTheme, TextInput};
use crate::picker::state::{PickerData, PickerEntry, PickerHead, PickerHeadColumn, PickerLayout};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const PICKER_ROW_MARKER_WIDTH: usize = 2;
const PICKER_HEAD_COLUMN_GAP_WIDTH: usize = 1;
const TRUNCATION_PREFIX: &str = "...";

/// Colors used by [`render_picker`]. `Default` uses terminal-neutral colors so
/// the picker looks reasonable on any palette; override fields to match your app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerTheme {
    pub background: Color,
    pub foreground: Color,
    pub secondary: Color,
    pub border: Color,
}

impl Default for PickerTheme {
    fn default() -> Self {
        Self {
            background: Color::Reset,
            foreground: Color::Reset,
            secondary: Color::Cyan,
            border: Color::DarkGray,
        }
    }
}

pub fn centered_picker_area(area: Rect) -> Rect {
    let width = (area.width.saturating_mul(88) / 100)
        .clamp(56, 156)
        .min(area.width);
    let height = (area.height.saturating_mul(78) / 100)
        .clamp(12, 32)
        .min(area.height);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}

pub fn render_picker<M>(
    f: &mut Frame,
    area: Rect,
    theme: &PickerTheme,
    picker: &mut PickerData<M>,
    focused: bool,
) {
    if area.width < 20 || area.height < 6 {
        return;
    }

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.background)),
        area,
    );

    let border_style = border_style(theme, focused);

    if should_render_single_pane(area, picker.layout) {
        render_results_pane(f, area, theme, picker, border_style);
        return;
    }

    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(49),
            Constraint::Length(0),
            Constraint::Percentage(51),
        ])
        .split(area);

    let left_area = pane_chunks[0];
    let gap_area = pane_chunks[1];
    let right_area = pane_chunks[2];

    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.background)),
        gap_area,
    );

    render_results_pane(f, left_area, theme, picker, border_style);
    render_preview_pane(f, right_area, theme, picker, border_style);
}

/// Like [`render_picker`], but renders the right-hand preview pane through
/// `render_custom_preview`. The closure receives the preview area and the
/// (immutable) picker, and owns whatever it draws there. When the picker
/// collapses to a single pane the custom preview is not shown.
pub fn render_picker_with_custom_preview<M, F>(
    f: &mut Frame,
    area: Rect,
    theme: &PickerTheme,
    picker: &mut PickerData<M>,
    focused: bool,
    render_custom_preview: F,
) where
    F: FnOnce(&mut Frame, Rect, &PickerData<M>),
{
    if area.width < 20 || area.height < 6 {
        return;
    }

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.background)),
        area,
    );

    let border_style = border_style(theme, focused);

    if should_render_single_pane(area, picker.layout) {
        render_results_pane(f, area, theme, picker, border_style);
        return;
    }

    let pane_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(49),
            Constraint::Length(0),
            Constraint::Percentage(51),
        ])
        .split(area);

    let left_area = pane_chunks[0];
    let gap_area = pane_chunks[1];
    let right_area = pane_chunks[2];

    f.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.background)),
        gap_area,
    );

    render_results_pane(f, left_area, theme, picker, border_style);
    render_custom_preview(f, right_area, picker);
}

fn border_style(theme: &PickerTheme, focused: bool) -> Style {
    if focused {
        Style::default().fg(theme.foreground).bg(theme.background)
    } else {
        Style::default().fg(theme.border).bg(theme.background)
    }
}

fn should_render_single_pane(area: Rect, layout: PickerLayout) -> bool {
    matches!(layout, PickerLayout::SinglePane) || area.width < 84
}

fn render_results_pane<M>(
    f: &mut Frame,
    area: Rect,
    theme: &PickerTheme,
    picker: &mut PickerData<M>,
    border_style: Style,
) {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.background));

    if let Some(title) = &picker.title {
        block = block.title(Line::from(Span::styled(
            format!("[{}]", title),
            Style::default().fg(theme.secondary).bg(theme.background),
        )));
    }

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if inner_area.width < 8 || inner_area.height < 4 {
        return;
    }

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(inner_area);

    let input_area = render_query_line(f, layout[0], theme, picker);
    render_results_list(f, layout[1], theme, picker);
    render_suggestions_dropdown(
        f,
        f.area(),
        suggestion_anchor(input_area),
        &DefaultCanvasTheme,
        picker.input_ref(),
    );
}

fn suggestion_anchor(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        width: area.width.saturating_sub(1),
        ..area
    }
}

fn render_query_line<M>(
    f: &mut Frame,
    area: Rect,
    theme: &PickerTheme,
    picker: &mut PickerData<M>,
) -> Rect {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(8)])
        .split(area);

    let query_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.border).bg(theme.background))
        .padding(Padding::left(1))
        .style(Style::default().bg(theme.background));
    let placeholder = picker.query_placeholder.clone();
    picker.input_mut().set_placeholder(placeholder);
    let input_widget = TextInput::default()
        .block(query_block.clone())
        .suggestion_style(Style::default().fg(theme.secondary).bg(theme.background))
        .style(Style::default().fg(theme.foreground).bg(theme.background));
    f.render_stateful_widget(input_widget, chunks[0], picker.input_mut());
    let (cx, cy) = picker.input_ref().cursor(chunks[0], Some(&query_block));
    f.set_cursor_position((cx, cy));

    let counter = Paragraph::new(picker.count_label())
        .alignment(Alignment::Right)
        .style(Style::default().fg(theme.secondary).bg(theme.background))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(theme.border).bg(theme.background))
                .style(Style::default().bg(theme.background)),
        );
    f.render_widget(counter, chunks[1]);
    chunks[0]
}

fn render_results_list<M>(f: &mut Frame, area: Rect, theme: &PickerTheme, picker: &PickerData<M>) {
    let mut list_inner = area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if list_inner.width == 0 || list_inner.height == 0 {
        return;
    }

    let head_widths = picker
        .head
        .as_ref()
        .map(|head| resolve_head_widths(head, list_inner.width as usize));

    if let (Some(head), Some(widths)) = (picker.head.as_ref(), head_widths.as_deref()) {
        let header_area = Rect {
            height: 1,
            ..list_inner
        };
        let header_style = Style::default().fg(theme.foreground).bg(theme.background);
        f.render_widget(
            Paragraph::new(Line::from(picker_head_spans(head, widths, header_style))),
            header_area,
        );
        list_inner.y = list_inner.y.saturating_add(1);
        list_inner.height = list_inner.height.saturating_sub(1);
        if list_inner.height == 0 {
            return;
        }
    }

    let items: Vec<ListItem<'_>> = picker
        .visible_entries()
        .map(|(filtered_index, entry)| {
            let is_selected = picker.selected_filtered_index == Some(filtered_index);
            let style = if is_selected {
                Style::default().fg(theme.secondary).bg(theme.background)
            } else {
                Style::default().fg(theme.foreground).bg(theme.background)
            };
            let marker = if is_selected { "> " } else { "  " };

            let spans = if let (Some(head), Some(widths)) =
                (picker.head.as_ref(), head_widths.as_deref())
            {
                picker_head_entry_spans(head, widths, entry, marker, style)
            } else {
                vec![
                    Span::styled(marker, style),
                    kind_label_span(entry, style),
                    Span::styled(entry.label.as_str(), style),
                ]
            };

            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(picker.selected_filtered_index);
    let list = List::new(items)
        .highlight_style(Style::default().bg(theme.background))
        .style(Style::default().bg(theme.background));
    f.render_stateful_widget(list, list_inner, &mut list_state);
}

fn picker_head_spans(head: &PickerHead, widths: &[usize], style: Style) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled("  ".to_string(), style)];
    let header_style = style.add_modifier(Modifier::UNDERLINED);
    for (index, (column, width)) in head
        .columns
        .iter()
        .zip(widths.iter().copied())
        .enumerate()
    {
        spans.push(Span::styled(
            truncate_right_with_ellipsis(&column.label, width),
            header_style,
        ));
        let label_width = UnicodeWidthStr::width(column.label.as_str()).min(width);
        if label_width < width {
            spans.push(Span::styled(" ".repeat(width - label_width), style));
        }
        if index + 1 < head.columns.len() {
            spans.push(Span::styled(
                " ".repeat(PICKER_HEAD_COLUMN_GAP_WIDTH),
                style,
            ));
        }
    }
    spans
}

fn picker_head_entry_spans(
    head: &PickerHead,
    widths: &[usize],
    entry: &PickerEntry,
    marker: &'static str,
    style: Style,
) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled(marker.to_string(), style)];
    for (index, (column, width)) in head
        .columns
        .iter()
        .zip(widths.iter().copied())
        .enumerate()
    {
        spans.push(Span::styled(
            format_head_cell(entry_head_value(entry, column), width),
            style,
        ));
        if index + 1 < head.columns.len() {
            spans.push(Span::styled(
                " ".repeat(PICKER_HEAD_COLUMN_GAP_WIDTH),
                style,
            ));
        }
    }
    spans
}

fn resolve_head_widths(head: &PickerHead, row_width: usize) -> Vec<usize> {
    let gap_width = head
        .columns
        .len()
        .saturating_sub(1)
        .saturating_mul(PICKER_HEAD_COLUMN_GAP_WIDTH);
    let mut remaining = row_width
        .saturating_sub(PICKER_ROW_MARKER_WIDTH)
        .saturating_sub(gap_width);
    let mut widths = Vec::with_capacity(head.columns.len());
    let mut flex_indices = Vec::new();

    for (index, column) in head.columns.iter().enumerate() {
        if column.width == 0 {
            widths.push(0);
            flex_indices.push(index);
            continue;
        }

        let width = column.width.min(remaining);
        widths.push(width);
        remaining = remaining.saturating_sub(width);
    }

    if flex_indices.is_empty() {
        return widths;
    }

    let base = remaining / flex_indices.len();
    let mut extra = remaining % flex_indices.len();
    for index in flex_indices {
        let add = usize::from(extra > 0);
        if extra > 0 {
            extra -= 1;
        }
        widths[index] = base + add;
    }

    widths
}

fn entry_head_value<'a>(entry: &'a PickerEntry, column: &PickerHeadColumn) -> &'a str {
    if column.field_key == "label" {
        return entry.label.as_str();
    }

    entry
        .fields
        .iter()
        .find(|field| field.key == column.field_key)
        .map(|field| field.value.as_str())
        .unwrap_or("")
}

fn format_head_cell(value: &str, width: usize) -> String {
    let mut cell = truncate_right_with_ellipsis(value, width);
    let cell_width = UnicodeWidthStr::width(cell.as_str());
    if cell_width < width {
        cell.push_str(&" ".repeat(width - cell_width));
    }
    cell
}

fn truncate_right_with_ellipsis(value: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    if UnicodeWidthStr::width(value) <= width {
        return value.to_string();
    }

    let ellipsis_width = UnicodeWidthStr::width(TRUNCATION_PREFIX);
    if width <= ellipsis_width {
        return rightmost_width(value, width);
    }

    let suffix = rightmost_width(value, width - ellipsis_width);
    format!("{TRUNCATION_PREFIX}{suffix}")
}

fn rightmost_width(value: &str, width: usize) -> String {
    let mut result = String::new();
    let mut current_width = 0;

    for ch in value.chars().rev() {
        let char_width = ch.width().unwrap_or(0);
        if current_width + char_width > width {
            break;
        }
        result.insert(0, ch);
        current_width += char_width;
    }

    result
}

/// If an entry carries a `kind_label` field, render it as a `[KIND]` prefix.
/// This is a soft convention: entries without the field render nothing extra.
fn kind_label_span<'a>(entry: &PickerEntry, style: Style) -> Span<'a> {
    let Some(kind) = entry
        .fields
        .iter()
        .find(|field| field.key == "kind_label")
        .map(|field| field.value.trim())
        .filter(|value| !value.is_empty())
    else {
        return Span::raw("");
    };

    Span::styled(format!("[{}] ", kind.to_uppercase()), style)
}

fn render_preview_pane<M>(
    f: &mut Frame,
    area: Rect,
    theme: &PickerTheme,
    picker: &PickerData<M>,
    border_style: Style,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.background));
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    if inner_area.width == 0 || inner_area.height == 0 {
        return;
    }

    let preview_inner = inner_area.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });
    if preview_inner.width == 0 || preview_inner.height == 0 {
        return;
    }

    let preview_text = picker
        .selected_entry()
        .filter(|entry| !entry.preview_lines.is_empty())
        .map(|entry| {
            Text::from(
                entry
                    .preview_lines
                    .iter()
                    .map(|line| {
                        Line::from(Span::styled(
                            line.as_str(),
                            Style::default().fg(theme.foreground).bg(theme.background),
                        ))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap_or_else(|| Text::from(picker.empty_preview.clone()));

    let preview = Paragraph::new(preview_text)
        .style(Style::default().bg(theme.background))
        .wrap(Wrap { trim: false });
    f.render_widget(preview, preview_inner);
}
