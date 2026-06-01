use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, area: Rect) {
    let mut lines = Vec::new();

    section(&mut lines, "Navigation");
    keys(
        &mut lines,
        &[
            ("Tab / Shift+Tab", "cycle focus (canvas fields -> buttons)"),
            ("j / k  or  h / l", "move focus; inside a widget, move within it"),
            ("g f | g e | g ?", "jump to Form / Editor / Help  (multi-key chord)"),
            (":", "open the command palette  (try :f  :e  :?  :q)"),
            ("Ctrl+C", "quit"),
        ],
    );

    section(&mut lines, "Form page — fields");
    keys(
        &mut lines,
        &[
            ("i", "edit the focused field (Name / Email / Role)"),
            ("Esc", "leave the field back to NORMAL"),
            ("Tab / Shift+Tab", "next / previous field while editing"),
        ],
    );

    section(&mut lines, "Form page — Role suggestions");
    keys(
        &mut lines,
        &[
            ("(edit Role)", "the role list opens automatically; type to filter"),
            ("Up / Down", "move the highlighted role"),
            ("Enter / Tab", "accept the highlighted role"),
            ("Esc", "close the list without choosing"),
            ("Ctrl+p/n/y", "alt: move up / down / accept"),
        ],
    );

    section(&mut lines, "Form page — Login");
    keys(
        &mut lines,
        &[
            ("Login button", "opens a dialog previewing the POST data"),
            ("Up/Down or h/l", "move between [Post] and [Cancel]"),
            ("Enter", "confirm the highlighted choice"),
            ("Esc", "dismiss the dialog"),
        ],
    );

    section(&mut lines, "Editor page");
    keys(
        &mut lines,
        &[
            ("Enter", "enter the textarea; then i edits, Esc leaves"),
            ("j / k", "while just a stop, jump straight to the buttons"),
        ],
    );

    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help ")),
        area,
    );
}

fn section(lines: &mut Vec<Line<'static>>, title: &str) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        title.to_string(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
}

fn keys(lines: &mut Vec<Line<'static>>, rows: &[(&str, &str)]) {
    lines.extend(
        rows.iter()
            .map(|(key, desc)| Line::from(format!("  {key:<18}{desc}"))),
    );
}
