# Core Concepts

## The Four Types

`tui-pages` is generic over four application-defined types:

```rust
TuiPages<V, A, S, O, M>
//  │    │  │  │  │  └─ Modal payload (default: ())
//  │    │  │  │  └─ Overlay type (default: ())
//  │    │  │  └─ Application state
//  │    │  └─ Action enum (your commands)
//  │    └─ View/Page enum
```

| Type | Your Responsibility | Examples |
|------|---------------------|----------|
| `V` | Define views/pages | `Home`, `Settings`, `Editor` |
| `A` | Define actions | `Save`, `Delete`, `Quit` |
| `S` | Application state | Database connections, config |
| `O` | Overlay identifiers | Dialog IDs, picker types |
| `M` | Modal cursor data | Selection indices |

## PageSpec

Each view maps to a `PageSpec` that defines its interaction model:

```rust
PageSpec {
    focus_targets: Vec<FocusTarget>,  // Navigable elements
    modes: Vec<ModeId>,               // Active input modes
    accepts_text_input: bool,         // Allow typing in this view
}
```

## TuiEffect

When your action handler wants the runtime to do something, it returns a `TuiEffect`:

```rust
pub enum TuiEffect<V, O, M> {
    None,
    Focus(FocusIntent<O, M>),         // Move focus
    Navigate(V),                       // Switch view
    NextBuffer, PreviousBuffer,        // Buffer navigation
    CloseBuffer,                      // Close current buffer
    SplitPane(PaneSplit),             // Split view
    ClosePane,                        // Close current pane
    NextPane, PreviousPane,           // Pane navigation
    RefreshPage,                      // Request redraw
    Quit,                             // Exit application
}
```

## ActionOutcome

Your handler returns `ActionOutcome` which wraps effects:

```rust
// Single effect
ActionOutcome::effect(TuiEffect::Quit)

// Multiple effects
ActionOutcome::effects([TuiEffect::Focus(FocusIntent::Next), TuiEffect::RefreshPage])

// No effects
ActionOutcome::none()
```

## Mode System

Modes allow different keybindings in different contexts:

```rust
// Built-in modes shipped by the runtime
modes::GENERAL   // Default page-navigation mode
modes::NORMAL    // Like vim normal mode (read-only navigation in fields)
modes::INSERT    // Like vim insert mode (typing into a text field)
modes::SELECT    // Like vim select mode (highlighting / selection)
modes::COMMAND   // Command bar (`:`) is open
modes::GLOBAL    // Active in all modes
modes::COMMON    // Shared bindings — active alongside `nor` and `sel`
```

`GENERAL`, `NORMAL`, `INSERT`, and `SELECT` are managed automatically by the
runtime. `COMMON` is for bindings you want available in both `nor` and `sel`.

### Custom modes for your own components

A `ModeId` is just a string key — nothing in the runtime is hardcoded to a
specific component. Define a mode for any UI you build (a picker, a command
palette, a sidebar) and register bindings for it the same way:

```rust
// Your component's mode — owned by your app, not the library.
const PICKER: ModeId = ModeId::borrowed("picker");

builder
    .bind(PICKER, "j", Action::PickerDown)
    .bind(PICKER, "k", Action::PickerUp)
    .bind(PICKER, "enter", Action::PickerSelect);

// Activate it for the page/overlay where the picker is open:
PageSpec::new().modes(vec![modes::GLOBAL, PICKER])
```

The library exposes the mechanism; you supply the concrete modes.

## FocusWrap Policy

Controls behavior at list boundaries:

```rust
pub enum FocusWrap {
    Clamp,  // Stop at first/last (default)
    Wrap,   // Cycle around
}
```

Applies uniformly to page focus, section items, modal items, buffers, and panes.
