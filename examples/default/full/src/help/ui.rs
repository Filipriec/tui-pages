use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect) {
    let keys = [
        ("Tab / Shift+Tab", "cycle focus"),
        ("Enter", "select (enter section / pick item / activate button)"),
        ("Esc", "leave section / close palette"),
        ("j / k  or  down / up", "move within a section"),
        ("g h | g n | g ?", "jump to Home / Notes / Help  (multi-key chord)"),
        ("[  ]  x", "prev / next / close buffer"),
        ("Ctrl+S / Ctrl+D", "split pane vertical / horizontal"),
        ("Ctrl+N / Ctrl+W", "next pane / close pane"),
        (":", "open command palette  (try :h :n :? :q)"),
        ("Ctrl+C", "quit"),
    ];

    let mut lines = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(keys.iter().map(|(key, desc)| Line::from(format!("{key:<22}{desc}"))));

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help ")),
        area,
    );
}
