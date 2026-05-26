//! Page rendering implementations.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    prelude::Stylize,
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};
use tui_pages::FocusTarget;

use crate::{AppState, AppView, Tui};

/// Render the current page based on the active view.
pub fn render(f: &mut Frame, area: Rect, tui: &Tui, state: &AppState) {
    let view = tui.current_view();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Title/status bar
            Constraint::Min(0),      // Main content
            Constraint::Length(3),   // Help bar
        ])
        .split(area);

    // Header
    render_header(f, chunks[0], *view, tui, state);

    // Main content area
    match view {
        AppView::Home => render_home(f, chunks[1], tui, state),
        AppView::Settings => render_settings(f, chunks[1], tui, state),
        AppView::Form => render_form(f, chunks[1], tui, state),
        AppView::Info => render_info(f, chunks[1], tui, state),
    }

    // Footer with key hints
    render_footer(f, chunks[2], *view, state);
}

fn render_header(
    f: &mut Frame,
    area: Rect,
    view: AppView,
    tui: &Tui,
    _state: &AppState,
) {
    let title = match view {
        AppView::Home => "tui-pages Demo | Home",
        AppView::Settings => "tui-pages Demo | Settings",
        AppView::Form => "tui-pages Demo | Form",
        AppView::Info => "tui-pages Demo | Info",
    };

    let mut header = Paragraph::new(title)
        .block(Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded))
        .style(Style::new().on_dark_gray())
        .alignment(Alignment::Center);

    // Show current focus if any
    if let Some(focus) = tui.focus.current() {
        let focus_str = format!(" [Focus: {:?}] ", focus);
        header = Paragraph::new(format!("{} {}", title, focus_str))
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .style(Style::new().on_dark_gray())
            .alignment(Alignment::Center);
    }

    f.render_widget(header, area);

    // Show buffer history
    let history: Vec<String> = tui.buffer.history.iter().map(|v| format!("{:?}", v)).collect();
    let history_str = format!("{:?}", history);
    let history_widget = Paragraph::new(format!("Buffers: {}", history_str))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Left);
    let history_area = Rect {
        x: area.x.saturating_add(area.width.saturating_sub(20)),
        y: area.y,
        width: area.width.min(20),
        height: area.height,
    };
    f.render_widget(history_widget, history_area);
}

fn render_home(
    f: &mut Frame,
    area: Rect,
    tui: &Tui,
    state: &AppState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(area);

    // Welcome message
    let welcome = Paragraph::new("Welcome to tui-pages Demo!\n\nThis demonstrates keyboard-driven TUI navigation.")
        .style(Style::new().italic())
        .alignment(Alignment::Center);
    f.render_widget(welcome, chunks[0]);

    // Navigation buttons
    render_buttons(f, chunks[1], tui, state, &[
        ("Go to Settings", AppView::Settings, 0),
        ("Go to Form", AppView::Form, 1),
        ("Command Palette", AppView::Home, 2),
    ]);
}

fn render_settings(
    f: &mut Frame,
    area: Rect,
    _tui: &Tui,
    _state: &AppState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // Left panel: options
    let options = List::new([
        ListItem::new("[ ] Option A"),
        ListItem::new("[x] Option B"),
        ListItem::new("[ ] Option C"),
    ])
    .block(Block::default()
        .title("Settings")
        .borders(Borders::ALL))
    .style(Style::new());

    f.render_widget(options, chunks[0]);

    // Right panel: info
    let info = Paragraph::new("Use Tab to move between options.\nPress Enter to toggle.\n\nPress Ctrl+h to go home.")
        .block(Block::default()
            .title("Help")
            .borders(Borders::ALL))
        .style(Style::new().fg(Color::DarkGray));

    f.render_widget(info, chunks[1]);
}

fn render_form(
    f: &mut Frame,
    area: Rect,
    tui: &Tui,
    state: &AppState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Name field
    let name_field = Paragraph::new(format!("Name: {}", state.form_name))
        .block(Block::default()
            .title("Name")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded))
        .style(if some_button_focused(tui, 0) {
            Style::new().on_green()
        } else {
            Style::new()
        });

    f.render_widget(name_field, chunks[0]);

    // Email field
    let email_field = Paragraph::new("Email: ")
        .block(Block::default()
            .title("Email")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded))
        .style(if some_button_focused(tui, 1) {
            Style::new().on_green()
        } else {
            Style::new()
        });

    f.render_widget(email_field, chunks[1]);

    // Action buttons
    let actions = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(chunks[2]);

    let submit_style = if some_button_focused(tui, 0) {
        Style::new().bold().on_green()
    } else {
        Style::new().bold()
    };

    let cancel_style = if some_button_focused(tui, 1) {
        Style::new().bold().on_red()
    } else {
        Style::new().bold()
    };

    let submit = Paragraph::new("Submit")
        .block(Block::default().borders(Borders::ALL))
        .style(submit_style)
        .alignment(Alignment::Center);

    let cancel = Paragraph::new("Cancel")
        .block(Block::default().borders(Borders::ALL))
        .style(cancel_style)
        .alignment(Alignment::Center);

    f.render_widget(submit, actions[0]);
    f.render_widget(cancel, actions[1]);
}

fn render_info(
    f: &mut Frame,
    area: Rect,
    _tui: &Tui,
    state: &AppState,
) {
    let info_text = vec![
        "tui-pages Library Features:",
        "",
        "- Mode-based keybindings (general, normal, insert, global)",
        "- Chord sequences (e.g., Ctrl+x s)",
        "- Focus management between interactive elements",
        "- Command palette with aliases",
        "- View/buffer history navigation",
        "- Overlay support (command bar, palettes)",
        "- Dialog and picker abstractions",
        "- Pane splits",
        "",
        "Keyboard shortcuts:",
        "- Tab/Shift+Tab: Navigate focus",
        "- Enter/Space: Activate",
        "- j/k: Normal mode navigation",
        "- Ctrl+h: Go home",
        "- Ctrl+q: Quit",
        "- Escape: Go back",
        "- ?: Command palette",
        "",
        &format!("Saved: {}", state.saved),
    ]
    .join("\n");

    let info = Paragraph::new(info_text)
        .block(Block::default()
            .title("About tui-pages")
            .borders(Borders::ALL))
        .alignment(Alignment::Left);

    f.render_widget(info, area);
}

fn render_footer(f: &mut Frame, area: Rect, view: AppView, state: &AppState) {
    let hints = match view {
        AppView::Home => "Tab: Next | Enter: Select | h: Home | ?: Palette | Ctrl+q: Quit",
        AppView::Settings => "Tab: Next | j/k: Navigate | Ctrl+h: Home | Ctrl+q: Quit",
        AppView::Form => "Tab: Next | Enter: Submit | j/k: Move | Ctrl+h: Home | Ctrl+q: Quit",
        AppView::Info => "Ctrl+h: Home | Ctrl+q: Quit",
    };

    let footer = Paragraph::new(hints)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::new().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    f.render_widget(footer, area);

    // Show message if any
    if let Some(msg) = &state.message {
        let msg_widget = Paragraph::new(msg.as_str())
            .style(Style::new().fg(Color::Yellow))
            .alignment(Alignment::Center);
        if area.height > 1 {
            let message_area = Rect {
                x: area.x,
                y: area.y.saturating_add(1),
                width: area.width,
                height: 1,
            };
            f.render_widget(msg_widget, message_area);
        }
    }
}

fn render_buttons(
    f: &mut Frame,
    area: Rect,
    tui: &Tui,
    _state: &AppState,
    buttons: &[(&str, AppView, usize)],
) {
    let len = buttons.len();
    let constraints: Vec<Constraint> = (0..len)
        .map(|_| Constraint::Percentage(100 / len as u16))
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (i, (_, _, btn_idx)) in buttons.iter().enumerate() {
        let focused = matches!(
            tui.focus.current(),
            Some(FocusTarget::Button(idx)) if idx == *btn_idx
        );

        let style = if focused {
            Style::new().bold().on_green()
        } else {
            Style::new()
        };

        let label = buttons[i].0;
        let button = Paragraph::new(label)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded))
            .style(style)
            .alignment(Alignment::Center);

        f.render_widget(button, chunks[i]);
    }
}

/// Helper to check if a button has focus.
fn some_button_focused(tui: &Tui, idx: usize) -> bool {
    matches!(
        tui.focus.current(),
        Some(FocusTarget::Button(i)) if i == idx
    )
}
