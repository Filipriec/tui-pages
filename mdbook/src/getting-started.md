# Getting Started

## Minimal Example

Here's the smallest working application:

```rust
use crossterm::event::KeyEvent;
use tui_pages::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppAction {
    FocusNext,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppView {
    Home,
    Settings,
}

struct AppState;
struct Handler;

impl TuiActionHandler<AppView, AppAction, AppState> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: AppAction,
        _ctx: ActionContext<AppView>,
        _state: &mut AppState,
    ) -> Result<ActionOutcome<AppView>, Self::Error> {
        Ok(match action {
            AppAction::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            AppAction::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

fn page_spec(
    view: &AppView,
    _state: &AppState,
    _focus: Option<&FocusTarget>,
) -> PageSpec {
    match view {
        AppView::Home => PageSpec::new()
            .focus_targets(vec![FocusTarget::Button(0), FocusTarget::Button(1)]),
        AppView::Settings => PageSpec::new()
            .focus_targets(vec![FocusTarget::Button(0)]),
    }
}

fn main() {
    let mut runtime = TuiPages::<AppView, AppAction, AppState>::builder(AppView::Home)
        .pages(page_spec)
        .handler(Handler)
        .bind(modes::GENERAL, "tab", AppAction::FocusNext)
        .command("quit", ["q"], AppAction::Quit)
        .build();

    let mut state = AppState;

    // Your event loop
    loop {
        let key = /* get key event */;

        match runtime.handle_key(key, &mut state) {
            TuiPagesOutput { status: TuiPagesStatus::ActionHandled, effects, .. } => {
                // Apply effects (handled automatically by TuiPages)
            },
            TuiPagesOutput { status: TuiPagesStatus::Waiting(hints), .. } => {
                // Show key sequence hints
            },
            _ => {}
        }
    }
}
```

## Builder Pattern

The `TuiPagesBuilder` provides a fluent API:

```rust
let mut runtime = TuiPages::<View, Action, State>::builder(initial_view)
    // Required
    .pages(page_function)    // PageProvider: maps views to PageSpec
    .handler(Handler)        // TuiActionHandler implementation

    // Optional
    .focus_wrap(FocusWrap::Wrap)  // Clamp or Wrap at list ends
    .command_timeout(1000)        // ms before incomplete command expires

    // Key bindings
    .bind(modes::GENERAL, "tab", Action::FocusNext)
    .bind(modes::INSERT, "esc", Action::EnterNormalMode)

    // Commands
    .command("quit", ["q"], Action::Quit)
    .command("save", ["s"], Action::Save)

    .build();
```

## Event Loop Integration

```rust
use crossterm::event::{Event, KeyEvent, KeyEventKind};

loop {
    if let Event::Key(KeyEvent { kind: KeyEventKind::Press, .. , code }) = event::read()? {
        match tui.handle_key(code, &mut state)? {
            TuiPagesOutput { quit_requested: true, .. } => break,
            TuiPagesOutput { status: TuiPagesStatus::Waiting(hints), .. } => {
                // Display hints for partial key sequences
            },
            TuiPagesOutput { status: TuiPagesStatus::TextInput(chord), .. } => {
                // Handle text input (when accepts_text_input is true)
            },
            _ => {}
        }
    }
}
```
