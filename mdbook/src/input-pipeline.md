# Input Pipeline

The input pipeline handles keyboard events and converts them to actions or text input.

## KeyChord

Represents a keyboard input combining key code and modifiers:

```rust
use tui_pages::KeyChord;

// From crossterm event
let chord = KeyChord::from_event(&key_event);

// Parsed from string
let chord = "C-s".parse::<KeyChord>().unwrap(); // Ctrl+S
let chord = "M-a".parse::<KeyChord>().unwrap(); // Alt+A
```

## Binding Syntax

Key bindings use a simple string format:

| Format | Example | Meaning |
|--------|---------|---------|
| `key` | `tab` | Single key |
| `C-key` | `C-s` | Ctrl + key |
| `A-key` | `A-x` | Alt + key |
| `S-key` | `S-tab` | Shift + key |
| `M-key` | `M-e` | Meta/Super + key |

## Mode-Based Keymaps

Key bindings are associated with modes:

```rust
// General mode bindings
.bind(modes::GENERAL, "tab", Action::FocusNext)
.bind(modes::GENERAL, "s", Action::Save)
.bind(modes::GENERAL, "C-q", Action::Quit)

// Insert mode bindings
.bind(modes::INSERT, "esc", Action::EnterNormalMode)
.bind(modes::INSERT, "C-c", Action::Copy)

// Global bindings (active in all modes)
.bind(modes::GLOBAL, "C-z", Action::Undo)
```

## PipelineResponse

The input pipeline returns `PipelineResponse<A>`:

```rust
pub enum PipelineResponse<A> {
    /// Key binding matched - execute the action
    Execute(A),

    /// Plain text input (when accepts_text_input is true)
    Type(KeyChord),

    /// Partial sequence match - show hints
    Wait(Vec<InputHint>),

    /// Sequence expired or invalid
    Cancel,
}
```

## Multi-Key Sequences

Support chord sequences like vim (e.g., `g` then `h` for help):

```rust
// Register a sequence
.bind(modes::GENERAL, "g h", Action::ShowHelp)  // "g h" = g followed by h
.bind(modes::GENERAL, "C-x C-c", Action::ForceQuit)

// Hints are shown while waiting for the second key
// If timeout expires, sequence cancels
```

## Text Input Mode

When a view accepts text input:

```rust
PageSpec::new()
    .accepts_text_input(true)
    .modes(vec![modes::INSERT])
```

In text input mode, plain character keys return `PipelineResponse::Type(chord)` instead of executing bindings. Mode-specific bindings (like `esc`) still work.

## Parsing Bindings

Utility functions for parsing binding strings:

```rust
use tui_pages::{parse_binding, try_parse_binding};

// Safe parsing with error
if let Ok(chord) = try_parse_binding("C-x") { ... }

// Panicking version
let chord = parse_binding("C-s");
```
