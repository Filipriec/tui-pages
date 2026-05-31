#![cfg(feature = "canvas")]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{
    canvas::{self, AppMode, DataProvider, FormEditor},
    ActionContext, ActionOutcome, CanvasAction, CanvasDispatchOutcome, FocusTarget, PageFocusBuilder,
    PageSpec, TuiActionHandler, TuiPages, TuiPagesStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum View {
    Form,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Canvas(CanvasAction),
}

impl From<CanvasAction> for Action {
    fn from(action: CanvasAction) -> Self {
        Action::Canvas(action)
    }
}

#[derive(Default)]
struct State {
    actions: Vec<Action>,
}

struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        state.actions.push(action);
        Ok(ActionOutcome::none())
    }
}

#[derive(Debug)]
struct Provider {
    values: Vec<String>,
}

impl DataProvider for Provider {
    fn field_count(&self) -> usize {
        self.values.len()
    }

    fn field_name(&self, _index: usize) -> &str {
        "field"
    }

    fn field_value(&self, index: usize) -> &str {
        &self.values[index]
    }

    fn set_field_value(&mut self, index: usize, value: String) {
        self.values[index] = value;
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn canvas_defaults_route_typed_chars_to_canvas_actions() {
    let pages = |_view: &View, _state: &State, _focus: Option<&FocusTarget>| {
        PageSpec::new()
            .focus(PageFocusBuilder::new().canvas_field(0))
            .canvas_mode(AppMode::Edit)
    };

    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .pages(pages)
        .handler(Handler)
        .canvas_defaults()
        .build();
    let mut state = State::default();

    app.refresh_page(&state);
    let output = app.handle_key(key(KeyCode::Char('x')), &mut state).unwrap();

    assert_eq!(output.status, TuiPagesStatus::ActionHandled);
    assert_eq!(state.actions, vec![Action::Canvas(CanvasAction::InsertChar('x'))]);
}

#[test]
fn dispatch_action_turns_read_only_canvas_boundary_into_focus_intent() {
    let mut editor = FormEditor::new(Provider {
        values: vec!["one".to_string()],
    });

    let outcome: CanvasDispatchOutcome<(), ()> =
        canvas::dispatch_action(&mut editor, CanvasAction::MoveDown);

    assert!(matches!(
        outcome,
        CanvasDispatchOutcome::Focus(tui_pages::FocusIntent::ExitCanvasForward)
    ));
}
