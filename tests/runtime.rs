use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{
    modes, ActionContext, ActionOutcome, FocusIntent, FocusTarget, PageSpec, TuiActionHandler,
    TuiEffect, TuiPages, TuiPagesStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum View {
    Home,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Next,
    Settings,
    Quit,
}

#[derive(Default)]
struct State {
    handled: Vec<Action>,
}

#[derive(Clone, Copy)]
struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        state: &mut State,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        state.handled.push(action.clone());
        Ok(match action {
            Action::Next => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::Settings => ActionOutcome::effect(TuiEffect::Navigate(View::Settings)),
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn runtime_maps_user_actions_to_library_effects() {
    let pages = |view: &View, _state: &State, _focus: Option<&FocusTarget>| match view {
        View::Home => PageSpec::new()
            .focus_targets(vec![FocusTarget::Button(0), FocusTarget::Button(1)])
            .modes(vec![modes::GENERAL, modes::GLOBAL]),
        View::Settings => PageSpec::new()
            .focus_targets(vec![FocusTarget::Button(0)])
            .modes(vec![modes::GENERAL, modes::GLOBAL]),
    };

    let mut tui = TuiPages::<View, Action, State>::builder(View::Home)
        .pages(pages)
        .handler(Handler)
        .bind(modes::GENERAL, "tab", Action::Next)
        .bind(modes::GENERAL, "s", Action::Settings)
        .command("Quit", ["q", "quit"], Action::Quit)
        .build();

    let mut state = State::default();
    tui.refresh_page(&state);
    assert_eq!(tui.focus.current(), Some(FocusTarget::Button(0)));

    let output = tui.handle_key(key(KeyCode::Tab), &mut state).unwrap();
    assert_eq!(output.status, TuiPagesStatus::ActionHandled);
    assert_eq!(tui.focus.current(), Some(FocusTarget::Button(1)));

    tui.handle_key(key(KeyCode::Char('s')), &mut state).unwrap();
    assert_eq!(tui.current_view(), &View::Settings);
    assert_eq!(tui.focus.current(), Some(FocusTarget::Button(0)));

    let output = tui.submit_command("q", &mut state).unwrap();
    assert!(output.quit_requested);
    assert_eq!(
        state.handled,
        vec![Action::Next, Action::Settings, Action::Quit]
    );
}
