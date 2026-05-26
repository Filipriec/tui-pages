use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyChord {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub fn from_event(event: &KeyEvent) -> Self {
        let mut modifiers = event.modifiers;
        if event.code == KeyCode::BackTab {
            modifiers -= KeyModifiers::SHIFT;
        }
        Self {
            code: event.code,
            modifiers,
        }
    }

    pub fn display_string(&self) -> String {
        let mut out = String::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            out.push_str("Ctrl+");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            out.push_str("Alt+");
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            out.push_str("Shift+");
        }

        match self.code {
            KeyCode::Char(c) => out.push(c),
            KeyCode::Enter => out.push_str("Enter"),
            KeyCode::Tab => out.push_str("Tab"),
            KeyCode::BackTab => out.push_str("BackTab"),
            KeyCode::Backspace => out.push_str("Backspace"),
            KeyCode::Esc => out.push_str("Esc"),
            KeyCode::Up => out.push_str("Up"),
            KeyCode::Down => out.push_str("Down"),
            KeyCode::Left => out.push_str("Left"),
            KeyCode::Right => out.push_str("Right"),
            other => out.push_str(&format!("{other:?}")),
        }

        out
    }
}
