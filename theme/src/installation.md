# Installation

Add `tui-pages` to your `Cargo.toml`:

```toml
[dependencies]
tui-pages = "0.1"
crossterm = "0.28"
```

## Optional Features

### Serde Support

Enable serialization on `ModeId`, `KeyChord`, and `FocusWrap`:

```toml
tui-pages = { version = "0.1", features = ["serde"] }
```

### Built-in Dialog

Include the modal dialog renderer (pulls in ratatui):

```toml
tui-pages = { version = "0.1", features = ["dialog"] }
```

This adds:
- `DialogData<T>` — dialog content type
- `DialogResult<T>` — dialog result type
- `DialogTheme` — styling configuration
- `render_dialog()` — ratatui renderer
- `dialog::handle_key()` — event handler helper

## Dependencies

`tui-pages` depends on:
- `crossterm 0.28` — for key events
- `tracing` — for debug logging
- `ratatui 0.28` — only when `dialog` feature is enabled

## MSRV

The minimum supported Rust version is Rust 1.75+.
