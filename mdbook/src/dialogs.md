# Dialogs

The optional `dialog` feature provides a built-in modal dialog with ratatui rendering.

## Enabling the Feature

```toml
[dependencies]
tui-pages = { version = "0.1", features = ["dialog"] }
```

This pulls in `ratatui` as a dependency.

## DialogData

Define your dialog content:

```rust
use tui_pages::DialogData;

struct ConfirmDialog {
    title: String,
    message: String,
    items: Vec<String>,  // Selection items
}

impl DialogData for ConfirmDialog {
    type Result = usize;  // Selected item index

    fn title(&self) -> &str { &self.title }
    fn item(&self, index: usize) -> &str { &self.items[index] }
    fn item_count(&self) -> usize { self.items.len() }
}
```

## DialogResult

Result of dialog interaction:

```rust
pub enum DialogResult<T> {
    Selected(T),  // User selected item, contains result
    Cancelled,    // User dismissed with Esc
}
```

## DialogKey

Result of handling a key in dialog mode:

```rust
pub enum DialogKey {
    Consumed,                    // Key was consumed by dialog
    Ignored,                     // Key should be handled normally
    Resolved(DialogResult<T>),   // Dialog completed with result
}
```

## handle_key Helper

The dialog module provides a key handler:

```rust
use tui_pages::dialog::{self, DialogKey};

loop {
    match dialog::handle_key(&mut tui.focus, key)? {
        DialogKey::Ignored => {
            // No dialog active or key should pass through
            tui.handle_key(key, state)?;
        }
        DialogKey::Consumed => {
            // Dialog consumed the key, request redraw
            needs_redraw = true;
        }
        DialogKey::Resolved(result) => {
            // Dialog completed
            match result {
                DialogResult::Selected(index) => handle_selection(index),
                DialogResult::Cancelled => dismiss_dialog(),
            }
        }
    }
}
```

## DialogTheme

Customize dialog appearance:

```rust
use tui_pages::DialogTheme;

let theme = DialogTheme {
    border_color: Color::Blue,
    selected_color: Color::Yellow,
    title_color: Color::Cyan,
    // ... customize styling
};

// Pass to renderer
render_dialog(frame, dialog, theme, area);
```

## render_dialog

Draw the dialog using ratatui:

```rust
use tui_pages::render_dialog;

fn ui(frame: &mut Frame, tui: &TuiPages<View, Action, State, (), ()>) {
    // Draw your normal UI
    // ...

    // Draw dialog if active
    if let Some(dialog) = active_dialog() {
        let area = centered_rect(60, 40, frame.size());
        render_dialog(frame, dialog, theme, area);
    }
}
```

## Opening a Dialog

```rust
fn handle_action(&mut self, action: Action, ctx: ActionContext<View>, state: &mut State) {
    match action {
        Action::OpenConfirmDialog => {
            // Set up dialog in focus
            let dialog = ConfirmDialog {
                title: "Confirm".into(),
                message: "Choose an option".into(),
                items: vec!["Yes".into(), "No".into()],
            };
            tui.focus.open_modal(dialog, 3);
            Ok(ActionOutcome::none())
        }
    }
}
```

## Dialog Conventions

The dialog module enforces standard bindings:

| Key | Action |
|-----|--------|
| `Tab` / `Arrow keys` | Navigate items |
| `Enter` | Select current item |
| `Esc` | Cancel/dismiss |
| `1-9` | Direct selection |
