use anyhow::Result;
use crossterm::event::Event;
use ratatui::{backend::CrosstermBackend, Terminal};
use tui_pages::canvas;
use tui_pages::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Form,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Canvas(canvas::CanvasAction),
    Select,
    Quit,
}

impl From<canvas::CanvasAction> for Action {
    fn from(action: canvas::CanvasAction) -> Self {
        Self::Canvas(action)
    }
}

#[derive(Debug)]
struct Contact {
    values: Vec<String>,
}

impl Default for Contact {
    fn default() -> Self {
        Self {
            values: vec!["Ada".to_string(), "ada@example.test".to_string()],
        }
    }
}

impl canvas::DataProvider for Contact {
    fn field_count(&self) -> usize {
        self.values.len()
    }

    fn field_name(&self, index: usize) -> &str {
        match index {
            0 => "Name",
            1 => "Email",
            _ => "",
        }
    }

    fn field_value(&self, index: usize) -> &str {
        &self.values[index]
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.values[index] = value;
    }
}

struct State {
    editor: canvas::FormEditor<Contact>,
    message: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            editor: canvas::FormEditor::new(Contact::default()),
            message: "i edits, Esc exits edit mode, Tab leaves the form, Ctrl+C quits".to_string(),
        }
    }
}

struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        ctx: ActionContext<View>,
        state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
            Action::Select => match ctx.focus {
                Some(FocusTarget::Button(0)) => ActionOutcome::effect(TuiEffect::Quit),
                _ => ActionOutcome::none(),
            },
            Action::Canvas(action) => match canvas::dispatch_action(&mut state.editor, action) {
                canvas::CanvasDispatchOutcome::Focus(intent) => {
                    ActionOutcome::effect(TuiEffect::Focus(intent))
                }
                canvas::CanvasDispatchOutcome::Applied(result) => {
                    state.message = match result {
                        canvas::ActionResult::Success => "form handled the key".to_string(),
                        canvas::ActionResult::Message(message)
                        | canvas::ActionResult::Error(message) => message,
                        _ => "form handled the key".to_string(),
                    };
                    ActionOutcome::none()
                }
            },
        })
    }
}

fn page_spec(_view: &View, state: &State, focus: Option<&FocusTarget>) -> PageSpec {
    let spec = PageSpec::new().focus(PageFocusBuilder::new().canvas_field(0).button(0));
    if matches!(focus, Some(FocusTarget::Button(0))) {
        spec
    } else {
        spec.canvas_editor(&state.editor)
    }
}

fn build() -> TuiApp<View, Action, State, Handler> {
    TuiPages::builder(View::Form)
        .page_fn(page_spec)
        .handler(Handler)
        .canvas_defaults()
        .bind(modes::GENERAL, "enter", Action::Select)
        .bind(modes::GLOBAL, "ctrl+c", Action::Quit)
        .build()
}

fn main() -> Result<()> {
    let _guard = tui_pages::terminal::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut tui = build();
    let mut state = State::default();
    tui.refresh_page(&state);

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            let rows = ratatui::layout::Layout::vertical([
                ratatui::layout::Constraint::Length(7),
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(area);

            canvas::render_canvas_with_suggestions_default(
                frame,
                area,
                rows[0],
                &state.editor,
            );
            frame.render_widget(
                ratatui::widgets::Paragraph::new(state.message.as_str()).block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title(" status "),
                ),
                rows[1],
            );
            frame.render_widget(
                ratatui::widgets::Paragraph::new("Quit")
                    .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL)),
                rows[2],
            );
        })?;

        let Event::Key(key) = crossterm::event::read()? else {
            continue;
        };
        if tui.handle_key(key, &mut state)?.quit_requested {
            break;
        }
    }

    Ok(())
}
