#![cfg(feature = "canvas")]

//! Integration tests for canvas support in `tui-pages`.
//!
//! The `canvas` feature enables *full* canvas support in one switch (we do not
//! split it into sub-features), so these tests exercise both the `tui-pages`
//! glue (keymaps, typed-text routing, focus handoff, mode sync) and the
//! re-export surface for every canvas capability — GUI, suggestions,
//! validation, computed, textarea, textinput, keymap, and cursor style.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{
    canvas::{self, AppMode, DataProvider, FormEditor},
    ActionContext, ActionOutcome, CanvasAction, CanvasDispatchOutcome, FocusIntent, FocusTarget,
    PageFocusBuilder, PageSpec, TuiActionHandler, TuiPages, TuiPagesStatus,
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

impl Provider {
    fn new(values: &[&str]) -> Self {
        Self {
            values: values.iter().map(|s| s.to_string()).collect(),
        }
    }
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

fn edit_page(_v: &View, _s: &State, _f: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus(PageFocusBuilder::new().canvas_field(0))
        .canvas_mode(AppMode::Edit)
}

fn read_only_page(_v: &View, _s: &State, _f: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus(PageFocusBuilder::new().canvas_field(0))
        .canvas_mode(AppMode::ReadOnly)
}

// --- Glue: typed-text routing -------------------------------------------------

#[test]
fn canvas_defaults_route_typed_chars_to_canvas_actions() {
    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .page_fn(edit_page)
        .handler(Handler)
        .canvas_defaults()
        .build();
    let mut state = State::default();
    app.refresh_page(&state);

    let output = app.handle_key(key(KeyCode::Char('x')), &mut state).unwrap();

    assert_eq!(output.status, TuiPagesStatus::ActionHandled);
    assert_eq!(
        state.actions,
        vec![Action::Canvas(CanvasAction::InsertChar('x'))]
    );
}

#[test]
fn shifted_chars_route_as_text_but_modified_chars_do_not() {
    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .page_fn(edit_page)
        .handler(Handler)
        .canvas_defaults()
        .build();
    let mut state = State::default();
    app.refresh_page(&state);

    // Shift+A is still plain text input.
    let shifted = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
    app.handle_key(shifted, &mut state).unwrap();

    // Ctrl+A is not a text char; with no binding for it the pipeline does not
    // turn it into an InsertChar.
    let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    let out = app.handle_key(ctrl_a, &mut state).unwrap();

    assert_eq!(
        state.actions,
        vec![Action::Canvas(CanvasAction::InsertChar('A'))]
    );
    // Ctrl+A produced no canvas insert action.
    assert_ne!(out.status, TuiPagesStatus::ActionHandled);
}

#[test]
fn typed_text_chord_helper_maps_plain_chars_only() {
    use tui_pages::KeyChord;

    let plain = canvas::text_chord_to_canvas_action(KeyChord::new(
        KeyCode::Char('z'),
        KeyModifiers::empty(),
    ));
    assert_eq!(plain, Some(CanvasAction::InsertChar('z')));

    let with_ctrl = canvas::text_chord_to_canvas_action(KeyChord::new(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL,
    ));
    assert_eq!(with_ctrl, None);
}

// --- Glue: keymaps ------------------------------------------------------------

#[test]
fn normal_mode_keymaps_drive_canvas_movement_actions() {
    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .page_fn(read_only_page)
        .handler(Handler)
        .canvas_defaults()
        .build();
    let mut state = State::default();
    app.refresh_page(&state);

    // `j` in read-only navigation maps to MoveDown via the default keymaps.
    app.handle_key(key(KeyCode::Char('j')), &mut state).unwrap();
    // `i` enters edit mode.
    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();

    assert_eq!(
        state.actions,
        vec![
            Action::Canvas(CanvasAction::MoveDown),
            Action::Canvas(CanvasAction::EnterEditMode),
        ]
    );
}

// --- Glue: focus handoff at canvas boundaries --------------------------------

#[test]
fn dispatch_action_turns_read_only_bottom_boundary_into_forward_intent() {
    let mut editor = FormEditor::new(Provider::new(&["one"]));

    let outcome: CanvasDispatchOutcome<(), ()> =
        canvas::dispatch_action(&mut editor, CanvasAction::MoveDown);

    assert!(matches!(
        outcome,
        CanvasDispatchOutcome::Focus(FocusIntent::ExitCanvasForward)
    ));
}

#[test]
fn dispatch_action_turns_read_only_top_boundary_into_backward_intent() {
    let mut editor = FormEditor::new(Provider::new(&["one", "two"]));

    // Already at field 0, moving up exits backward.
    let outcome: CanvasDispatchOutcome<(), ()> =
        canvas::dispatch_action(&mut editor, CanvasAction::MoveUp);

    assert!(matches!(
        outcome,
        CanvasDispatchOutcome::Focus(FocusIntent::ExitCanvasBackward)
    ));
}

#[test]
fn dispatch_action_applies_interior_movement_without_handoff() {
    let mut editor = FormEditor::new(Provider::new(&["one", "two", "three"]));

    // Moving down from field 0 stays inside the canvas (Applied, not Focus).
    let outcome: CanvasDispatchOutcome<(), ()> =
        canvas::dispatch_action(&mut editor, CanvasAction::MoveDown);

    assert!(matches!(outcome, CanvasDispatchOutcome::Applied(_)));
    assert_eq!(editor.current_field(), 1);
}

#[test]
fn boundary_helper_maps_exits_to_intents() {
    let fwd: FocusIntent =
        canvas::focus_intent_for_boundary(canvas::BoundaryExit::Bottom);
    let back: FocusIntent =
        canvas::focus_intent_for_boundary(canvas::BoundaryExit::Top);
    assert_eq!(fwd, FocusIntent::ExitCanvasForward);
    assert_eq!(back, FocusIntent::ExitCanvasBackward);
}

// --- Glue: PageSpec mode sync -------------------------------------------------

#[test]
fn page_spec_canvas_editor_tracks_editor_mode() {
    let mut editor = FormEditor::new(Provider::new(&["a"]));

    let read_only: PageSpec = PageSpec::new().canvas_editor(&editor);
    assert!(!read_only.accepts_text_input);

    editor.enter_edit_mode();
    let editing: PageSpec = PageSpec::new().canvas_editor(&editor);
    assert!(editing.accepts_text_input);
}

#[test]
fn app_mode_to_modes_mapping_is_stable() {
    use tui_pages::modes;

    assert_eq!(canvas::mode_for_app_mode(AppMode::Edit), modes::INSERT);
    assert_eq!(canvas::mode_for_app_mode(AppMode::Highlight), modes::SELECT);
    assert_eq!(canvas::mode_for_app_mode(AppMode::ReadOnly), modes::NORMAL);
    assert!(canvas::accepts_text_input(AppMode::Edit));
    assert!(!canvas::accepts_text_input(AppMode::ReadOnly));
}

// --- Editor: typed editing through CanvasAction::execute ----------------------

#[test]
fn editor_executes_insert_and_delete_actions() {
    let mut editor = FormEditor::new(Provider::new(&[""]));
    editor.execute(CanvasAction::EnterEditMode);
    editor.execute(CanvasAction::InsertChar('h'));
    editor.execute(CanvasAction::InsertChar('i'));
    assert_eq!(editor.current_text(), "hi");

    editor.execute(CanvasAction::DeleteBackward);
    assert_eq!(editor.current_text(), "h");
}

// --- TextInput surface (proves textinput re-export works end to end) ----------

#[test]
fn text_input_round_trips_text_and_strips_pasted_newlines() {
    use canvas::{TextInputEventOutcome, TextInputState};

    let mut input = TextInputState::<canvas::TextInputProvider>::from_text("hello");
    assert_eq!(input.text(), "hello");

    input.set_text("");
    let outcome = input.paste("multi\nline\rpaste");
    assert_eq!(outcome, TextInputEventOutcome::Handled);
    // Newlines/carriage returns are filtered out of single-line input.
    assert_eq!(input.text(), "multilinepaste");
}

#[test]
fn text_input_accepts_suggestion_suffix() {
    use canvas::{TextInputEventOutcome, TextInputState};

    let mut input = TextInputState::<canvas::TextInputProvider>::from_text("doc");
    input.set_suggestion_suffix("uments");
    let outcome = input.accept_suggestion_suffix();

    assert_eq!(outcome, TextInputEventOutcome::Handled);
    assert_eq!(input.text(), "documents");
    assert_eq!(input.suggestion_suffix(), None);
}

// --- Full-surface re-export proof ---------------------------------------------

/// Compile-time proof that the single `canvas` feature re-exports every canvas
/// surface through `tui_pages::canvas`. If any surface stopped being enabled by
/// the unified feature, this test would fail to compile.
#[test]
fn every_canvas_surface_is_reachable_through_tui_pages() {
    // Base + result types.
    let _action = CanvasAction::InsertChar('a');
    let _result: canvas::ActionResult = canvas::ActionResult::Success;
    let _: Option<canvas::EditorState> = None;

    // GUI: themes, display options, renderers.
    let _theme = canvas::DefaultCanvasTheme::default();
    let _opts = canvas::CanvasDisplayOptions::default();
    let _overflow: Option<canvas::OverflowMode> = None;
    // `render_canvas_default` draws into a ratatui `Frame`; we only prove the
    // path resolves (rendering needs a live terminal/frame to call).
    let _render = canvas::render_canvas_default::<Provider>;

    // Cursor style.
    let _cursor: Option<canvas::CursorManager> = None;

    // Suggestions.
    let _trigger = canvas::SuggestionTrigger::WhenFieldStarts;
    let _query = canvas::SuggestionQuery::whole_field("q");
    let _: Option<canvas::SuggestionItem> = None;

    // Validation.
    let _: Option<canvas::ValidationConfig> = None;
    let _: Option<canvas::ValidationRule> = None;
    let _: Option<canvas::DisplayMask> = None;
    let _: Option<canvas::ValidationSummary> = None;

    // Computed.
    let _: Option<canvas::ComputedState> = None;

    // Textarea / textinput widgets.
    let _: Option<canvas::TextAreaState> = None;
    let _: Option<canvas::TextInputProvider> = None;

    // Keymap + host handoff. `CanvasKeyMap` is built from mode->binding maps.
    let empty = std::collections::HashMap::new();
    let _km = canvas::CanvasKeyMap::from_mode_maps(&empty, &empty, &empty);
    let _: Option<canvas::KeyEventOutcome> = None;
    let _: Option<canvas::HostKeyEventOutcome> = None;

    // Crossterm session helpers.
    let _: Option<canvas::CrosstermInputOptions> = None;

    // Touch a value so the bindings are not all dead.
    assert!(_action.is_editing_action());
    assert!(_result.is_success());
    let _ = (_theme, _opts, _trigger, _query, _km, _render);
}
