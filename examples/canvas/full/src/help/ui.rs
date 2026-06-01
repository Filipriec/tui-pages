use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect) {
    let keys = [
        ("Tab / Shift+Tab", "cycle focus (canvas → buttons)"),
        ("j / k  or  h / l", "move focus; inside a widget, move within it"),
        ("Enter", "activate button / enter the textarea"),
        ("i / Esc", "in the textarea: insert mode / leave it"),
        ("g f | g e | g ?", "jump to Form / Editor / Help  (multi-key chord)"),
        (":", "open command palette  (try :f :e :? :q)"),
        ("Ctrl+C", "quit"),
    ];

    let mut lines = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    lines.extend(keys.iter().map(|(key, desc)| Line::from(format!("{key:<20}{desc}"))));

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help ")),
        area,
    );
}
