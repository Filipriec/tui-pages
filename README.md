# tui-pages

Reusable architecture primitives for keyboard-driven TUI applications.

The crate extracts the working structure from the client app into generic,
application-owned building blocks:

- input pipeline: key chords, sequence tracking, key maps, and pipeline responses
- command pipeline: command aliases, hints, and command resolution
- focus manager: page focus registration, overlays, dialogs, pickers, sections, and canvas handoff
- action routing: pipeline response to page/canvas/global logic
- navigation: buffer history, panes, router synchronization, and focus registration

Applications keep their own page enum, app state, render code, side effects,
canvas/editor actions, dialogs, and picker data. `tui-pages` only owns the
coordination model.

## Minimal Shape

```rust
use crossterm::event::KeyEvent;
use tui_pages::{
    ActionDispatcher, BufferState, DefaultActionDecider, FocusManager, InputPipeline,
    InputRegistry, NavigationCoordinator,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppAction {
    Save,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppView {
    Home,
}

fn handle_key(
    key: KeyEvent,
    pipeline: &mut InputPipeline<AppAction>,
    dispatcher: &ActionDispatcher<DefaultActionDecider>,
    focus: &FocusManager,
    view: &AppView,
) {
    let response = pipeline.process(key, &["general", "global"], false);
    let current_focus = focus.current().unwrap_or(tui_pages::FocusTarget::Button(0));
    let resolution = dispatcher.resolve_intent(response, &current_focus, view);
    // Execute the resolution in your app.
}
```

## Architecture

The original architecture diagrams are kept in `docs/new_system`.
