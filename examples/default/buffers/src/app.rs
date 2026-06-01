// Demonstrates the buffer / pane API of `tui-pages`.
//
// Everything here talks to the library; ui.rs only *reads* the resulting
// `BufferState`. The buffer model has two layers:
//
//   * the **buffer history** — the list of open views you cycle through, and
//   * the **workspace** — how the active area is split into panes.
//
// The handler never touches `BufferState` directly: it returns `TuiEffect`s
// (Navigate / NextBuffer / SplitPane / …) and the runtime applies them to
// `tui.buffer`. That is the whole point — you describe intent, the runtime
// owns the buffer state machine.

use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Editor,
    Terminal,
    Docs,
}

impl View {
    pub fn name(self) -> &'static str {
        match self {
            View::Editor => "Editor",
            View::Terminal => "Terminal",
            View::Docs => "Docs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Standard navigation actions provided by the keybinding presets
    /// (buffer / pane cycling, splits, quit).
    Nav(NavigationAction),
    Open(View),
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
}

pub type App = TuiApp<View, Action, (), Handler>;

pub struct Handler;

impl TuiActionHandler<View, Action, ()> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        _state: &mut (),
    ) -> Result<ActionOutcome<View>, Self::Error> {
        let effect = match action {
            Action::Open(view) => TuiEffect::Navigate(view),
            Action::Nav(nav) => nav.to_effect(),
        };
        Ok(ActionOutcome::effect(effect))
    }
}

// This example is about buffers, not focus, so the page declares no focus
// targets — just the modes its key bindings live in.
fn page_spec(_view: &View, _state: &(), _focus: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new().modes(vec![modes::GENERAL, modes::GLOBAL])
}

pub fn build() -> App {
    let mut app = TuiPages::builder(View::Editor)
        // Buffer and pane cycling honor the wrap policy too: with Wrap, NextBuffer
        // off the last buffer returns to the first (try it at the ends).
        .focus_wrap(FocusWrap::Wrap)
        .page_fn(page_spec)
        .handler(Handler)
        // Vim preset covers the standard navigation: next/prev/close buffer
        // (]/[/x), next/prev/close pane (ctrl+n/ctrl+p/ctrl+w), vertical and
        // horizontal splits (ctrl+s/ctrl+d), and quit (ctrl+c). The 1/2/3
        // shortcuts to open a specific view are app-specific and stay below.
        .vim_defaults()
        .vim_navigation_defaults()
        .bind(modes::GENERAL, "1", Action::Open(View::Editor))
        .bind(modes::GENERAL, "2", Action::Open(View::Terminal))
        .bind(modes::GENERAL, "3", Action::Open(View::Docs))
        .build();
    app.refresh_page(&());
    app
}
