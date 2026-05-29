# tui-pages

Opinionated coordination runtime for page-based, keyboard-driven TUI
applications.

The crate extracts the working structure from the client app into generic,
application-owned building blocks:

- input pipeline: key chords, sequence tracking, key maps, and pipeline responses
- command pipeline: command aliases, hints, and command resolution
- focus manager: page focus registration, overlays, dialogs, pickers, sections, and canvas handoff
- runtime facade: key events and command input to user actions and library effects
- navigation: buffer history, panes, router synchronization, and focus registration

Applications keep their own action enum, page enum, app state, render code,
side effects, canvas/editor actions, dialogs, and picker data. `tui-pages`
only owns the coordination model: input sequences, command resolution, focus,
overlays, navigation, buffers, and panes.

User actions are opaque to the library. Keymaps and commands resolve to the
application's action enum, and the application handler returns `TuiEffect`
values when it wants the runtime to move focus, navigate, switch buffers, open
overlays, or quit.

## Minimal Shape

```rust
use crossterm::event::KeyEvent;
use tui_pages::{
    modes, ActionContext, ActionOutcome, FocusIntent, FocusTarget, PageSpec,
    TuiActionHandler, TuiEffect, TuiPages,
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppAction {
    FocusNext,
    OpenSettings,
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
            AppAction::OpenSettings => ActionOutcome::effect(TuiEffect::Navigate(AppView::Settings)),
            AppAction::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

fn run_one_key(key: KeyEvent, state: &mut AppState) {
    let mut runtime = TuiPages::<AppView, AppAction, AppState>::builder(AppView::Home)
        .pages(|view: &AppView, _state: &AppState, _focus: Option<&FocusTarget>| match view {
            AppView::Home => PageSpec::new()
                .focus_targets(vec![FocusTarget::Button(0), FocusTarget::Button(1)])
                .modes(vec![modes::GENERAL, modes::GLOBAL]),
            AppView::Settings => PageSpec::new()
                .focus_targets(vec![FocusTarget::Button(0)])
                .modes(vec![modes::GENERAL, modes::GLOBAL]),
        })
        .handler(Handler)
        .bind(modes::GENERAL, "tab", AppAction::FocusNext)
        .bind(modes::GENERAL, "s", AppAction::OpenSettings)
        .command("Quit", ["q", "quit"], AppAction::Quit)
        .build();

    let _ = runtime.handle_key(key, state);
}
```

## Architecture

See [`docs/architecture/architecture.md`](docs/architecture/architecture.md)
for the full design, flow diagrams, and the primitive layer.
