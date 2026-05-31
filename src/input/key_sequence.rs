use crate::input::KeyChord;
use crossterm::event::{KeyCode, KeyModifiers};
use std::fmt;

/// Why a binding string failed to parse. Returned by [`try_parse_key`] and
/// [`try_parse_binding`]; the lenient [`parse_key`] / [`parse_binding`] drop
/// the offending token instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseKeyError {
    /// The binding string was empty or contained no tokens.
    Empty,
    /// A token could not be recognised as a key (a typo, or a multi-character
    /// name that is not a known key). Carries the offending token.
    UnknownKey(String),
}

impl fmt::Display for ParseKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseKeyError::Empty => write!(f, "empty key binding"),
            ParseKeyError::UnknownKey(token) => write!(f, "unrecognised key token: {token:?}"),
        }
    }
}

impl std::error::Error for ParseKeyError {}

/// Parse a whitespace-separated chord sequence (e.g. `"ctrl+shift+x z"`),
/// silently dropping any token that does not parse.
///
/// Convenient for trusted, in-code bindings. For user-supplied config where a
/// typo should surface rather than vanish, use [`try_parse_binding`].
pub fn parse_binding(binding: &str) -> Vec<KeyChord> {
    binding
        .split_whitespace()
        .flat_map(parse_binding_token)
        .collect()
}

/// Parse a single chord token (e.g. `"ctrl+s"`), returning `None` if it is not
/// recognised. The strict variant is [`try_parse_key`].
pub fn parse_key(token: &str) -> Option<KeyChord> {
    try_parse_key(token).ok()
}

/// Parse a whitespace-separated chord sequence, returning [`ParseKeyError`] on
/// the first unrecognised token (or if the string has no tokens).
///
/// Use this when binding from a user-editable config or a remap UI so a typo
/// like `"ctrl+shft+x"` is reported instead of silently ignored.
pub fn try_parse_binding(binding: &str) -> Result<Vec<KeyChord>, ParseKeyError> {
    let chords = binding
        .split_whitespace()
        .map(try_parse_binding_token)
        .collect::<Result<Vec<_>, _>>()?;
    let chords = chords.into_iter().flatten().collect::<Vec<_>>();
    if chords.is_empty() {
        return Err(ParseKeyError::Empty);
    }
    Ok(chords)
}

fn parse_binding_token(token: &str) -> Vec<KeyChord> {
    try_parse_binding_token(token).unwrap_or_default()
}

fn try_parse_binding_token(token: &str) -> Result<Vec<KeyChord>, ParseKeyError> {
    let parts = token.split('+').collect::<Vec<_>>();
    if parts.len() <= 1 || is_modifier_sequence(&parts) {
        return try_parse_key(token).map(|key| vec![key]);
    }

    parts.into_iter().map(try_parse_key).collect()
}

fn is_modifier_sequence(parts: &[&str]) -> bool {
    parts
        .iter()
        .take(parts.len().saturating_sub(1))
        .all(|part| is_modifier(part))
}

fn is_modifier(part: &str) -> bool {
    matches!(
        part.to_ascii_lowercase().as_str(),
        "ctrl" | "control" | "c" | "alt" | "meta" | "m" | "shift" | "s"
    )
}

/// Parse a single chord token, returning [`ParseKeyError`] when it is not
/// recognised. The lenient variant is [`parse_key`].
pub fn try_parse_key(token: &str) -> Result<KeyChord, ParseKeyError> {
    let mut modifiers = KeyModifiers::empty();
    let mut key = token.trim();
    if key.is_empty() {
        return Err(ParseKeyError::Empty);
    }

    loop {
        let Some((prefix, rest)) = key.split_once('+') else {
            break;
        };

        match prefix.to_ascii_lowercase().as_str() {
            "ctrl" | "control" | "c" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" | "m" => modifiers |= KeyModifiers::ALT,
            "shift" | "s" => modifiers |= KeyModifiers::SHIFT,
            // Not a modifier — the remainder (including this prefix, e.g. the
            // literal `+` key) is the key code.
            _ => break,
        }
        key = rest;
    }

    let unknown = || ParseKeyError::UnknownKey(token.to_string());
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
        // Function keys `f1`..`f12`. A bare `f` falls through to the single
        // character branch below so it can still be bound as the letter.
        text if text.starts_with('f') && text.len() > 1 => {
            let number = text[1..].parse().map_err(|_| unknown())?;
            KeyCode::F(number)
        }
        text => {
            let mut chars = text.chars();
            let first = chars.next().ok_or_else(unknown)?;
            if chars.next().is_some() {
                return Err(unknown());
            }
            KeyCode::Char(first)
        }
    };

    let (code, modifiers) = if code == KeyCode::Tab && modifiers.contains(KeyModifiers::SHIFT) {
        (KeyCode::BackTab, modifiers - KeyModifiers::SHIFT)
    } else {
        (code, modifiers)
    };

    Ok(KeyChord::new(code, modifiers))
}
