use crate::input::KeyChord;
use crossterm::event::{KeyCode, KeyModifiers};

pub fn parse_binding(binding: &str) -> Vec<KeyChord> {
    binding
        .split_whitespace()
        .filter_map(parse_key)
        .collect()
}

pub fn parse_key(token: &str) -> Option<KeyChord> {
    let mut modifiers = KeyModifiers::empty();
    let mut key = token.trim();

    loop {
        let Some((prefix, rest)) = key.split_once('+') else {
            break;
        };

        match prefix.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "c" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" | "m" => modifiers |= KeyModifiers::ALT,
            "shift" | "s" => modifiers |= KeyModifiers::SHIFT,
            _ => break,
        }
        key = rest;
    }

    let code = match key.to_ascii_lowercase().as_str() {
        "enter" | "return" => KeyCode::Enter,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" | "bs" => KeyCode::Backspace,
        "space" => KeyCode::Char(' '),
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "page_up" => KeyCode::PageUp,
        "pagedown" | "page_down" => KeyCode::PageDown,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        text if text.starts_with('f') => {
            let number = text[1..].parse().ok()?;
            KeyCode::F(number)
        }
        text => {
            let mut chars = text.chars();
            let first = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(first)
        }
    };

    let (code, modifiers) = if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        (KeyCode::BackTab, modifiers - KeyModifiers::SHIFT)
    } else {
        (code, modifiers)
    };

    Some(KeyChord::new(code, modifiers))
}
