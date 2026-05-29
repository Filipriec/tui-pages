# tui-pages

> **Origin:** This crate was extracted from a production application and
> packaged into a standalone crate. The underlying code is handcrafted; the
> extraction and crate-packaging work was done with AI - Claude, GPT, Minimax.

Opinionated coordination runtime for page-based, keyboard-driven TUI
applications.

The crate extracts the working structure from the client app into generic,
application-owned building blocks:

- input pipeline: key chords, sequence tracking, key maps, and pipeline responses
- command pipeline: command aliases, hints, and command resolution
- focus manager: page focus registration, overlays, dialogs, pickers, sections, and canvas handoff
- runtime facade: key events and command input to user actions and library effects
- navigation: buffer history, panes, router synchronization, and focus registration

The core crate ships **no rendering** — you draw state with ratatui or anything
else. The one exception is the optional `dialog` feature, an opt-in built-in
modal dialog (content type, result type, and a ratatui renderer) that pulls in
ratatui only when enabled:

```toml
tui-pages = { version = "0.1", features = ["dialog"] }
```

A dialog is driven from your event loop with one helper — `dialog::handle_key`
applies the conventional bindings (Tab/arrows move, Enter selects, Esc
dismisses), closes the dialog, and hands you the result:

```rust
match dialog::handle_key(&mut tui.focus, key) {
    DialogKey::Ignored => { tui.handle_key(key, state)?; } // no dialog: normal input
    DialogKey::Consumed => {}                               // navigated; redraw
    DialogKey::Resolved(result) => apply(result, state),   // answered
}
```

Applications keep their own action enum, page enum, app state, render code,
side effects, canvas/editor actions, dialogs, and picker data. `tui-pages`
only owns the coordination model: input sequences, command resolution, focus,
overlays, navigation, buffers, and panes.

User actions are opaque to the library. Keymaps and commands resolve to the
application's action enum, and the application handler returns `TuiEffect`
values when it wants the runtime to move focus, navigate, switch buffers, open
overlays, or quit.

## Minimal Shape

`use tui_pages::prelude::*;` pulls in the runtime, the focus types, the
`PageFn` alias, and the `FocusController` trait (whose `apply_focus_intent`
method is otherwise invisible until the trait is in scope). With the `dialog`
feature it also re-exports the dialog content/result/theme/renderer and the
`dialog::*` driver helpers.

```rust
use crossterm::event::KeyEvent;
use tui_pages::prelude::*;

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
