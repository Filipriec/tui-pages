# Introduction

> **Origin:** This crate was extracted from a production application and packaged into a standalone crate.

`tui-pages` is an **opinionated coordination runtime** for keyboard-driven, page-based TUI applications. It provides the structural scaffolding for managing input handling, command resolution, focus management, and navigation — without dictating how you render your UI.

## Philosophy

The crate follows a strict separation of concerns:

| You Own | tui-pages Owns |
|---------|----------------|
| Action enum | Input sequences |
| View/Page enum | Command resolution |
| App state | Focus management |
| Rendering | Overlays |
| Side effects | Navigation |
| Dialogs | Buffers & panes |

**The core crate ships no rendering** — you draw state with [ratatui](https://ratatui.rs/) or anything else. The one exception is the optional `dialog` feature, an opt-in built-in modal dialog.

## Key Features

### Input Pipeline
- Mode-based keymaps (vi-like modal editing)
- Multi-key chord sequences (e.g., `g` then `h` for help)
- Key bindings resolve to your action enum

### Command Resolution
- Fuzzy prefix matching for command palettes
- Alias support with tab completion hints
- Configurable timeout for incomplete commands

### Focus Management
- Focus targets per page: buttons, sections, overlays, modals
- FocusWrap policy: clamp at ends or wrap around
- Canvas handoff: enter/exit focus areas while maintaining state

### Navigation & Buffers
- View history with buffer state
- Multi-pane support with split direction
- Workspace state management

### Dialogs (Optional)
Built-in modal dialog with ratatui renderer when the `dialog` feature is enabled.

## When to Use tui-pages

- Building a keyboard-driven TUI application
- Need structured focus management across multiple views
- Want mode-based keybindings (like vim)
- Need a command palette with fuzzy matching
- Managing multi-pane or tabbed interfaces

## When Not to Use

- Simple, single-view TUIs (use ratatui directly)
- Applications that don't need keyboard navigation
- Games or highly custom input models
