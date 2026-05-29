# Navigation & Buffers

Navigation manages view history, multi-pane layouts, and workspace state.

## BufferState

Tracks view history with an active index:

```rust
use tui_pages::BufferState;

// Create with initial view
let buffers = BufferState::new(initial_view);

// Navigate forward/backward
buffers.push(new_view);        // Add new view, advance
let prev = buffers.previous(); // Go back
let next = buffers.next();     // Go forward again

// Query state
buffers.current();             // Option<&View>
buffers.can_go_back();         // bool
buffers.can_go_forward();      // bool
buffers.len();                 // usize
```

## PaneSplit

Define how to split the view:

```rust
pub enum PaneSplit {
    Horizontal,  // Left/Right
    Vertical,    // Top/Bottom
}
```

## PaneSession

Manage multiple panes:

```rust
use tui_pages::{PaneSession, PaneId};

// Create a new pane session
let mut panes = PaneSession::new(initial_view, initial_view);

// Split current pane
panes.split(PaneSplit::Horizontal)?;
// Now has two panes side by side

// Navigate panes
panes.next_pane();
panes.previous_pane();
panes.close_current_pane();

// Access pane content
let active_id = panes.active_pane_id();
let view = panes.pane_view(active_id);
```

## WorkspaceState

Combines buffers and panes for full workspace management:

```rust
use tui_pages::WorkspaceState;

let workspace = WorkspaceState::new(initial_view);

// Navigation with buffer history
workspace.navigate(new_view)?;

// Pane management
workspace.split_active(PaneSplit::Vertical)?;
workspace.close_active_pane()?;
workspace.switch_pane(PaneId::new(2))?;
```

## NavigationCoordinator

Low-level navigation primitives:

```rust
use tui_pages::NavigationCoordinator;

// Create coordinator
let mut coordinator = NavigationCoordinator::new(initial_view);

// Handle navigation events
match event {
    NavigationEvent::BufferNext => coordinator.next_buffer(),
    NavigationEvent::BufferPrev => coordinator.previous_buffer(),
    NavigationEvent::CloseBuffer => coordinator.close_buffer()?,
    NavigationEvent::Split(split) => coordinator.split_pane(split)?,
    NavigationEvent::ClosePane => coordinator.close_pane()?,
}
```

## NavigationRouter

Sync navigation with focus registration:

```rust
use tui_pages::NavigationRouter;

impl NavigationRouter<View> for MyRouter {
    fn navigate_to(&mut self, view: View) -> Result<(), AppError> {
        // Called when navigation effect is triggered
        self.current_view = view;
        Ok(())
    }

    fn current_view(&self) -> &View {
        &self.current_view
    }
}
```

## TuiEffect Navigation

The runtime interprets these effects automatically:

```rust
ActionOutcome::effect(TuiEffect::Navigate(new_view))
ActionOutcome::effect(TuiEffect::NextBuffer)
ActionOutcome::effect(TuiEffect::PreviousBuffer)
ActionOutcome::effect(TuiEffect::CloseBuffer)
ActionOutcome::effect(TuiEffect::SplitPane(PaneSplit::Horizontal))
ActionOutcome::effect(TuiEffect::ClosePane)
ActionOutcome::effect(TuiEffect::NextPane)
ActionOutcome::effect(TuiEffect::PreviousPane)
```

## Focus Registration

Navigation coordinates with focus management:

```rust
// When navigating, focus state is preserved per-view
// Or reset based on page spec
tui.navigate_with(view, |focus| {
    focus.reset();  // Custom focus behavior
});
```
