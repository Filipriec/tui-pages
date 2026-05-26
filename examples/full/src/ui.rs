// Pure ratatui rendering. Reads tui-pages state to decide what to draw.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use tui_pages::{FocusTarget, InputHint};

use crate::app::{App, AppState, View, NOTES, NOTES_SECTION};

pub struct Status<'a, A> {
    pub waiting: Option<&'a [InputHint<A>]>,
}

pub fn render(frame: &mut Frame, tui: &App, state: &AppState, status: Status<'_, crate::app::Action>) {
    let area = frame.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tabs
            Constraint::Length(3), // buffer / pane strip
            Constraint::Min(0),    // body
            Constraint::Length(3), // status
        ])
        .split(area);

    render_tabs(frame, rows[0], tui);
    render_workspace(frame, rows[1], tui);
    render_body(frame, rows[2], tui, state);
    render_status(frame, rows[3], tui, state, status);

    if state.palette_open {
        render_palette(frame, area, state);
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, tui: &App) {
    let current = tui.current_view();
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
            tab("Home", *current == View::Home),
            Span::raw("  "),
            tab("Notes", *current == View::Notes),
            Span::raw("  "),
            tab("Help", *current == View::Help),
        ]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" views ")),
        area,
    );
}

fn render_workspace(frame: &mut Frame, area: Rect, tui: &App) {
    let buffers = tui
        .buffer
        .history
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i == tui.buffer.active_index {
                format!("*{:?}", v)
            } else {
                format!(" {:?}", v)
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    let panes = tui
        .buffer
        .panes()
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if i == tui.buffer.active_pane_index() {
                format!("*{}:{:?}", p.pane_id.0, p.view)
            } else {
                format!(" {}:{:?}", p.pane_id.0, p.view)
            }
        })
        .collect::<Vec<_>>()
        .join("  ");

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("buffers: {buffers}")),
            Line::from(format!("panes:   {panes}")),
        ])
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title(" workspace ")),
        area,
    );
}

fn render_body(frame: &mut Frame, area: Rect, tui: &App, state: &AppState) {
    match tui.current_view() {
        View::Home => render_home(frame, area, tui),
        View::Notes => render_notes(frame, area, tui, state),
        View::Help => render_help(frame, area),
    }
}

fn render_home(frame: &mut Frame, area: Rect, tui: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3), Constraint::Length(3)])
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

    render_button(frame, rows[1], "Open Notes", tui.focus.current(), 0);
    render_button(frame, rows[2], "Open Help", tui.focus.current(), 1);
}

fn render_notes(frame: &mut Frame, area: Rect, tui: &App, state: &AppState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let focus = tui.focus.current();
    let section_focused = matches!(
        focus,
        Some(FocusTarget::Section(NOTES_SECTION))
            | Some(FocusTarget::SectionItem { section: NOTES_SECTION, .. })
    );
    let active_item = match focus {
        Some(FocusTarget::SectionItem { section: NOTES_SECTION, item }) => Some(item),
        _ => state.selected_note,
    };

    let items = NOTES
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let mark = if Some(i) == state.selected_note { "[x]" } else { "[ ]" };
            ListItem::new(format!("{mark} {n}"))
        })
        .collect::<Vec<_>>();

    let mut list_state = ListState::default();
    list_state.select(active_item);

    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Notes ")
                    .borders(Borders::ALL)
                    .border_style(if section_focused {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    }),
            )
            .highlight_symbol("> ")
            .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        cols[0],
        &mut list_state,
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(cols[1]);

    render_button(frame, rows[0], "Back to Home", focus.clone(), 0);

    let detail = match state.selected_note {
        Some(i) => format!("Selected:\n\n  {}", NOTES[i]),
        None => "Press Enter on the list, then j/k, then Enter to pick a note.".to_string(),
    };
    frame.render_widget(
        Paragraph::new(detail)
            .block(Block::default().borders(Borders::ALL).title(" Detail ")),
        rows[1],
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Keybindings", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("Tab / Shift+Tab     cycle focus"),
        Line::from("Enter               select (enter section / pick item / activate button)"),
        Line::from("Esc                 leave section / close palette"),
        Line::from("j / k  or  ↓ / ↑    move within a section"),
        Line::from("g h | g n | g ?     jump to Home / Notes / Help  (multi-key chord)"),
        Line::from("[  ]  x             prev / next / close buffer"),
        Line::from("Ctrl+S / Ctrl+D     split pane vertical / horizontal"),
        Line::from("Ctrl+N / Ctrl+W     next pane / close pane"),
        Line::from(":                   open command palette  (try :h :n :? :q)"),
        Line::from("Ctrl+C              quit"),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help ")),
        area,
    );
}

fn render_button(frame: &mut Frame, area: Rect, label: &str, focus: Option<FocusTarget>, index: usize) {
    let focused = matches!(focus, Some(FocusTarget::Button(i)) if i == index);
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

fn render_status(
    frame: &mut Frame,
    area: Rect,
    tui: &App,
    state: &AppState,
    status: Status<'_, crate::app::Action>,
) {
    let focus_label = tui
        .focus
        .current()
        .map(|f| format!("{f:?}"))
        .unwrap_or_else(|| "None".into());

    let waiting = status.waiting.and_then(|hints| {
        if hints.is_empty() {
            None
        } else {
            let preview = hints
                .iter()
                .take(4)
                .map(|h| h.key.display_string())
                .collect::<Vec<_>>()
                .join(" / ");
            Some(format!("…waiting: {preview}"))
        }
    });

    let mut lines = vec![Line::from(format!("focus: {focus_label}"))];
    if let Some(w) = waiting {
        lines.push(Line::from(Span::styled(
            w,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
    } else if !state.message.is_empty() {
        lines.push(Line::from(state.message.as_str()));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title(" status ")),
        area,
    );
}

fn render_palette(frame: &mut Frame, area: Rect, state: &AppState) {
    let width = area.width.saturating_sub(10).min(60);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + area.height / 3;
    let rect = Rect::new(x, y, width, 3);

    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(format!(":{}", state.palette_input))
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" command palette — Enter to run, Esc to close ")
                    .border_style(Style::default().fg(Color::Yellow)),
            ),
        rect,
    );
}
