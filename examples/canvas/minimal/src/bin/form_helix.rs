use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use tui_pages::{canvas, canvas::DataProvider, prelude::*};

struct FormData {
    fields: Vec<(&'static str, String)>,
}

impl Default for FormData {
    fn default() -> Self {
        Self {
            fields: vec![
                ("Name", String::new()),
                ("Email", String::new()),
                ("Message", String::new()),
            ],
        }
    }
}

impl canvas::DataProvider for FormData {
    fn field_count(&self) -> usize {
        self.fields.len()
    }

    fn field_name(&self, index: usize) -> &str {
        self.fields[index].0
    }

    fn field_value(&self, index: usize) -> &str {
        &self.fields[index].1
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.fields[index].1 = value;
    }
}

struct State {
    editor: canvas::FormEditor<FormData>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            editor: canvas::FormEditor::new(FormData::default()),
        }
    }
}

impl canvas::CanvasWidgetState for State {
    fn canvas_form_editor_ref(&self, id: usize) -> Option<&dyn canvas::CanvasFormEditorHost> {
        (id == 0).then_some(&self.editor as &dyn canvas::CanvasFormEditorHost)
    }

    fn canvas_form_editor(&mut self, id: usize) -> Option<&mut dyn canvas::CanvasFormEditorHost> {
        (id == 0).then_some(&mut self.editor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Form,
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
    PageSpec::new().focus(PageFocusBuilder::new().canvas_fields(3))
}

fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Form)
        .page_fn(page_spec)
        .handler(Handler)
        .canvas_form_editor_with_preset(0, canvas::BuiltinCanvasKeybindingPreset::Helix)
        .helix_defaults()
        .build()
}

fn render(frame: &mut Frame, state: &State) {
    let chunks = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(frame.area());
    canvas::render_canvas_default(frame, chunks[0], state.editor.core());

    let status = format!(
        "mode: {:?}  field: {}/{}  -  Helix form through tui-pages",
        state.editor.mode(),
        state.editor.current_field() + 1,
        state.editor.data_provider().field_count(),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::raw(status)))
            .block(Block::default().borders(Borders::ALL).title("form_helix")),
        chunks[1],
    );
}

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = build();
    let mut state = State::default();
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| render(frame, &state))?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };
        if tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}
