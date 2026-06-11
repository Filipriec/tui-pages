use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
    backend::CrosstermBackend,
    widgets::Block,
    Frame, Terminal,
};
use tui_pages::{canvas, prelude::*};

struct State {
    textarea: canvas::TextAreaState<canvas::TextAreaProvider>,
    entered: bool,
}

impl Default for State {
    fn default() -> Self {
        let mut textarea = canvas::TextAreaState::from_text("A simple Helix textarea.\nType here.");
        textarea.use_wrap();
        textarea.use_default_commandline();
        Self {
            textarea,
            entered: false,
        }
    }
}

impl canvas::CanvasWidgetState for State {
    fn canvas_textarea_ref(
        &self,
        focus_index: usize,
    ) -> Option<&dyn canvas::CanvasTextAreaHost> {
        (focus_index == 0).then_some(&self.textarea as &dyn canvas::CanvasTextAreaHost)
    }

    fn canvas_textarea(
        &mut self,
        focus_index: usize,
    ) -> Option<&mut dyn canvas::CanvasTextAreaHost> {
        (focus_index == 0).then_some(&mut self.textarea)
    }

    fn canvas_textarea_entered(&mut self, focus_index: usize) -> Option<&mut bool> {
        (focus_index == 0).then_some(&mut self.entered)
    }

    fn canvas_textarea_entered_ref(&self, focus_index: usize) -> Option<&bool> {
        (focus_index == 0).then_some(&self.entered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Editor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Nav(NavigationAction),
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Self::Nav(value)
    }
}

struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        _state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::Nav(nav) => ActionOutcome::effect(nav.to_effect()),
        })
    }
}

fn page_spec(_view: &View, _state: &State, _focus: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new().focus(PageFocusBuilder::new().canvas_field(0))
}

fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Editor)
        .page_fn(page_spec)
        .handler(Handler)
        .canvas_textarea_widget_with_preset(0, canvas::BuiltinCanvasKeybindingPreset::Helix)
        .helix_defaults()
        .build()
}

fn render(frame: &mut Frame, state: &mut State) {
    let area = frame.area();
    let block = Block::bordered().title("textarea_helix_minimal");
    frame.render_stateful_widget(
        canvas::TextArea::default().block(block.clone()),
        area,
        &mut state.textarea,
    );
    let (x, y) = state.textarea.cursor_with_commandline(area, Some(&block));
    frame.set_cursor_position((x, y));
}

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = build();
    let mut state = State::default();
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| render(frame, &mut state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };
        if tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}
