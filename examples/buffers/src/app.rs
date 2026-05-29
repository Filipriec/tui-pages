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

use tui_pages::{prelude::*, PaneSplit};

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

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Open(View),         // -> TuiEffect::Navigate: open / switch to a buffer
    NextBuffer,         // -> cycle the buffer history forward
    PrevBuffer,         // -> cycle the buffer history backward
    CloseBuffer,        // -> drop the active buffer
    Split(PaneSplit),   // -> split the active pane
    NextPane,           // -> move focus to the next pane
    PrevPane,           // -> move focus to the previous pane
    ClosePane,          // -> close the active pane
    Quit,
}

pub type App = TuiPages<View, Action, (), PageFn<View, ()>, Handler>;

pub struct Handler;

impl TuiActionHandler<View, Action, ()> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        _state: &mut (),
    ) -> Result<ActionOutcome<View>, Self::Error> {
        // Each action maps to exactly one buffer/pane effect.
        let effect = match action {
            Action::Open(view) => TuiEffect::Navigate(view),
            Action::NextBuffer => TuiEffect::NextBuffer,
            Action::PrevBuffer => TuiEffect::PreviousBuffer,
            Action::CloseBuffer => TuiEffect::CloseBuffer,
            Action::Split(split) => TuiEffect::SplitPane(split),
            Action::NextPane => TuiEffect::NextPane,
            Action::PrevPane => TuiEffect::PreviousPane,
            Action::ClosePane => TuiEffect::ClosePane,
            Action::Quit => TuiEffect::Quit,
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
        .pages(page_spec as PageFn<View, ()>)
        .handler(Handler)
        // Open / switch buffers.
        .bind(modes::GENERAL, "1", Action::Open(View::Editor))
        .bind(modes::GENERAL, "2", Action::Open(View::Terminal))
        .bind(modes::GENERAL, "3", Action::Open(View::Docs))
        // Cycle the buffer history.
        .bind(modes::GENERAL, "tab", Action::NextBuffer)
        .bind(modes::GENERAL, "shift+tab", Action::PrevBuffer)
        .bind(modes::GENERAL, "w", Action::CloseBuffer)
        // Split / navigate / close panes.
        .bind(modes::GENERAL, "v", Action::Split(PaneSplit::Vertical))
        .bind(modes::GENERAL, "s", Action::Split(PaneSplit::Horizontal))
        .bind(modes::GENERAL, "o", Action::NextPane)
        .bind(modes::GENERAL, "p", Action::PrevPane)
        .bind(modes::GENERAL, "x", Action::ClosePane)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build();
    app.refresh_page(&());
    app
}
