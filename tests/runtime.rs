use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{
    ActionContext, ActionOutcome, FocusIntent, FocusTarget, KeyChord, NavigationAction, PageSpec,
    RuntimeContext, TuiActionHandler, TuiEffect, TuiPages, TuiPagesStatus, modes,
    navigation_action_outcome,
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
    Nav(NavigationAction),
}

impl From<NavigationAction> for Action {
    fn from(value: NavigationAction) -> Self {
        Action::Nav(value)
    }
}

#[derive(Default)]
struct State {
    handled: Vec<Action>,
    typed: Vec<KeyChord>,
}

#[cfg(feature = "canvas")]
impl tui_pages::canvas::CanvasWidgetState for State {}

#[derive(Clone, Copy)]
struct Handler;

impl TuiActionHandler<View, Action, State> for Handler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: Action,
        _ctx: ActionContext<View>,
        state: &mut State,
        _runtime: RuntimeContext<'_, Action>,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        state.handled.push(action.clone());
        Ok(match action {
            Action::Next => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::Settings => ActionOutcome::effect(TuiEffect::Navigate(View::Settings)),
            Action::Quit => ActionOutcome::effect(TuiEffect::Quit),
            Action::Nav(nav) => navigation_action_outcome(nav),
        })
    }

    fn handle_text(
        &mut self,
        chord: KeyChord,
        _ctx: ActionContext<View>,
        state: &mut State,
        _runtime: RuntimeContext<'_, Action>,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        state.typed.push(chord);
        Ok(ActionOutcome::none())
    }
}

#[test]
fn runtime_remap_navigation_preset_toml_changes_live_keymap_and_resets_sequence() {
    let pages = |_view: &View, _state: &State, _focus: Option<&FocusTarget>| {
        PageSpec::new()
            .focus_targets(vec![FocusTarget::Button(0)])
            .modes(vec![modes::GENERAL, modes::GLOBAL])
    };

    let mut tui = TuiPages::<View, Action>::builder(View::Home)
        .pages(pages)
        .handler(Handler)
        .helix_navigation_defaults()
        .build();
    let mut state = State::default();
    tui.refresh_page(&state);

    let output = tui.handle_key(key(KeyCode::Char('g')), &mut state).unwrap();
    assert!(matches!(output.status, TuiPagesStatus::Waiting(_)));
    assert!(tui.input.active());

    let remap = r#"
[general]
mode = "general"
next_buffer = ["n"]
"#;
    tui.remap_navigation_preset_toml(remap).unwrap();
    assert!(!tui.input.active());

    let output = tui.handle_key(key(KeyCode::Char('n')), &mut state).unwrap();
    assert_eq!(output.status, TuiPagesStatus::ActionHandled);
    assert_eq!(
        state.handled,
        vec![Action::Nav(NavigationAction::NextBuffer)]
    );
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn map_text_to_next(_chord: KeyChord) -> Option<Action> {
    Some(Action::Next)
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

    let mut tui = TuiPages::<View, Action>::builder(View::Home)
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

#[test]
fn text_mapper_does_not_hijack_non_canvas_text_targets() {
    let pages = |_view: &View, _state: &State, _focus: Option<&FocusTarget>| {
        PageSpec::new()
            .focus_targets(vec![FocusTarget::Overlay(())])
            .modes(vec![modes::INSERT, modes::GLOBAL])
            .accepts_text_input(true)
    };

    let mut tui = TuiPages::<View, Action>::builder(View::Home)
        .pages(pages)
        .handler(Handler)
        .text_input_mapper(map_text_to_next)
        .build();
    let mut state = State::default();
    tui.refresh_page(&state);

    let output = tui
        .handle_key(
            KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
            &mut state,
        )
        .unwrap();

    assert_eq!(output.status, TuiPagesStatus::TextHandled);
    assert!(state.handled.is_empty());
    assert_eq!(
        state.typed,
        vec![KeyChord::new(KeyCode::Char('A'), KeyModifiers::SHIFT)]
    );
}

#[cfg(feature = "command-line")]
#[test]
fn command_line_reservation_splits_bottom_row_from_page_area() {
    use ratatui::layout::Rect;

    let pages = |_view: &View, _state: &State, _focus: Option<&FocusTarget>| PageSpec::new();

    let tui = TuiPages::<View, Action>::builder(View::Home)
        .pages(pages)
        .handler(Handler)
        .reserve_command_line(true)
        .build();

    let areas = tui.render_areas(Rect::new(0, 0, 80, 24));

    assert_eq!(areas.page, Rect::new(0, 0, 80, 23));
    assert_eq!(areas.command_line, Some(Rect::new(0, 23, 80, 1)));
}

#[cfg(feature = "command-line")]
#[test]
fn command_line_reservation_gives_command_line_priority_on_tiny_area() {
    use ratatui::layout::Rect;

    let pages = |_view: &View, _state: &State, _focus: Option<&FocusTarget>| PageSpec::new();

    let tui = TuiPages::<View, Action>::builder(View::Home)
        .pages(pages)
        .handler(Handler)
        .reserve_command_line(true)
        .build();

    let areas = tui.render_areas(Rect::new(0, 0, 80, 1));

    assert_eq!(areas.page, Rect::new(0, 0, 80, 0));
    assert_eq!(areas.command_line, Some(Rect::new(0, 0, 80, 1)));
}

#[cfg(feature = "command-line")]
#[test]
fn command_line_area_is_absent_when_not_reserved() {
    use ratatui::layout::Rect;

    let pages = |_view: &View, _state: &State, _focus: Option<&FocusTarget>| PageSpec::new();

    let tui = TuiPages::<View, Action>::builder(View::Home)
        .pages(pages)
        .handler(Handler)
        .reserve_command_line(false)
        .build();

    let areas = tui.render_areas(Rect::new(0, 0, 80, 24));

    assert_eq!(areas.page, Rect::new(0, 0, 80, 24));
    assert_eq!(areas.command_line, None);
}
