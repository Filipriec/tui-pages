# Focus Management

Focus management tracks which UI element is currently selected and handles navigation.

## FocusTarget

Elements that can receive focus:

```rust
pub enum FocusTarget<O = ()> {
    Button(usize),        // Generic button (indexed)
    Overlay(O),           // Application-defined overlay
    ModalItem(usize),    // Item in a modal dialog
    Section {
        id: usize,        // Section identifier
        item_index: usize,
        item_count: usize,
    },
    Canvas,              // Text/canvas area
}
```

## FocusIntent

Actions that change focus:

```rust
pub enum FocusIntent<O, M> {
    Next,                     // Move to next target
    Prev,                     // Move to previous target
    Set(usize),               // Set focus to index
    Open(OverlayFocus<O, M>), // Open overlay/modal
    Close,                    // Close current overlay
    Toggle,                   // Toggle current focus
}
```

## FocusWrap

Navigation behavior at list boundaries:

```rust
// Builder configuration
TuiPages::builder(view)
    .focus_wrap(FocusWrap::Wrap)  // Cycle through targets

// Runtime change
tui.focus.set_focus_wrap(FocusWrap::Clamp);
```

## OverlayFocus

Overlays have two shapes:

```rust
pub enum OverlayFocus<O, M> {
    Simple(O),  // App-defined overlay
    Modal {
        data: M,      // Modal payload
        index: usize, // Selected item index
        count: usize, // Total items
    },
}
```

## FocusController Trait

Apply focus intents through the trait:

```rust
use tui_pages::FocusController;

impl FocusController for MyApp {
    fn apply_focus_intent(&mut self, intent: FocusIntent) {
        match intent {
            FocusIntent::Next => self.focus.next(),
            FocusIntent::Prev => self.focus.prev(),
            FocusIntent::Set(i) => self.focus.set_index(i),
            // ...
        }
    }
}
```

The `FocusController` trait comes into scope via the prelude.

## Canvas Handoff

Enter and exit canvas areas while maintaining page focus:

```rust
// In your action handler
fn handle_action(&mut self, action: Action, ctx: ActionContext<View>, state: &mut State) {
    match action {
        Action::EnterCanvas => {
            self.focus.enter_canvas();
            Ok(ActionOutcome::none())
        }
        Action::ExitCanvas => {
            self.focus.exit_canvas();
            Ok(ActionOutcome::none())
        }
        // Canvas receives key events, but page focus state is preserved
    }
}
```

## Focus Queries

Query the current focus state:

```rust
// Get current focus
let current = tui.focus.current();  // Option<FocusTarget>

// Check if in overlay
let has_overlay = tui.focus.has_overlay();

// Get current index
let index = tui.focus.index();
```
