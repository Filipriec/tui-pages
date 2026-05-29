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
// Predefined modes
modes::GENERAL   // Default mode
modes::NORMAL    // Like vim normal mode
modes::INSERT    // Like vim insert mode
modes::SELECT    // Like vim select mode
modes::PALETTE   // Command palette active
modes::PICKER    // Picker dialog active
modes::COMMAND   // Command input active
modes::GLOBAL    // Active in all modes
modes::COMMON    // Shared bindings
```

## FocusWrap Policy

Controls behavior at list boundaries:

```rust
pub enum FocusWrap {
    Clamp,  // Stop at first/last (default)
    Wrap,   // Cycle around
}
```

Applies uniformly to page focus, section items, modal items, buffers, and panes.
