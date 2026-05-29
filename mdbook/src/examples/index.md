# Examples

This section walks through complete examples demonstrating `tui-pages` usage.

## Example List

- [Minimal Example](./minimal.md) — Bare-bones TUI with two buttons
- [Full Example](./full.md) — Complete application with navigation and dialogs

## Running the Examples

```bash
# Minimal example
cargo run --example minimal

# Full example
cargo run --example full

# With dialog feature
cargo run --example minimal_dialog --features dialog
```

## Example Projects

The `examples/` directory in the repository contains runnable examples:

| Example | Description |
|---------|-------------|
| `minimal` | Basic two-button UI with Tab navigation |
| `buffers` | Multi-buffer navigation with history |
| `full` | Full application with all features |
| `minimal_dialog` | Dialog integration example |
