#![cfg(feature = "canvas")]

//! Integration tests for canvas support in `tui-pages`.
//!
//! The `canvas` feature enables *full* canvas support in one switch (we do not
//! split it into sub-features), so these tests exercise both the `tui-pages`
//! glue (keybindings, typed-text routing, focus handoff, mode sync) and the
//! re-export surface for every canvas capability — GUI, suggestions,
//! validation, computed, textarea, textinput, keybindings, and cursor style.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{
    ActionContext, ActionOutcome, CanvasAction, CanvasDispatchOutcome, FocusIntent, FocusTarget,
    PageFocusBuilder, PageSpec, TuiActionHandler, TuiEffect, TuiPages, TuiPagesStatus,
    canvas::{self, AppMode, DataProvider, FormEditor},
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum View {
    Form,
    Second,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Canvas(CanvasAction),
    FocusNext,
    FocusPrev,
    Activate,
    ShowModal,
    ClearOverlay,
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
        state.actions.push(action.clone());
        Ok(match action {
            Action::FocusNext => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Next)),
            Action::FocusPrev => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Prev)),
            Action::Activate => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::Activate)),
            Action::ShowModal => ActionOutcome::effect(TuiEffect::Focus(FocusIntent::ShowModal {
                data: (),
                count: 2,
            })),
            Action::ClearOverlay => {
                ActionOutcome::effect(TuiEffect::Focus(FocusIntent::ClearOverlay))
            }
            Action::Canvas(_) => ActionOutcome::none(),
        })
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
        .canvas_mode(AppMode::Ins)
}

fn read_only_page(_v: &View, _s: &State, _f: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus(PageFocusBuilder::new().canvas_field(0))
        .canvas_mode(AppMode::Nor)
}

fn mixed_focus_page(_v: &View, _s: &State, _f: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new()
        .focus(
            PageFocusBuilder::new()
                .canvas_field(0)
                .button(0)
                .section_with_items(7, 2)
                .canvas_field(1)
                .internal_canvas_field(99),
        )
        .canvas_mode(AppMode::Nor)
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

// --- Glue: keybindings --------------------------------------------------------

#[test]
fn normal_mode_keybindings_drive_canvas_movement_actions() {
    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .page_fn(read_only_page)
        .handler(Handler)
        .canvas_defaults()
        .build();
    let mut state = State::default();
    app.refresh_page(&state);

    // `j` in read-only navigation maps to MoveDown via the default keybindings.
    app.handle_key(key(KeyCode::Char('j')), &mut state).unwrap();
    // `i` enters edit mode.
    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    // Undo/redo come from the canvas crate's default vim bindings.
    app.handle_key(key(KeyCode::Char('u')), &mut state).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();

    assert_eq!(
        state.actions,
        vec![
            Action::Canvas(CanvasAction::MoveDown),
            Action::Canvas(CanvasAction::EnterEditMode),
            Action::Canvas(CanvasAction::Undo),
            Action::Canvas(CanvasAction::Redo),
        ]
    );
}

#[test]
fn suggestion_default_keybindings_route_to_canvas_actions() {
    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .page_fn(edit_page)
        .handler(Handler)
        .canvas_defaults()
        .build();
    let mut state = State::default();
    app.refresh_page(&state);

    app.handle_key(
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();

    assert_eq!(
        state.actions,
        vec![
            Action::Canvas(CanvasAction::TriggerSuggestions),
            Action::Canvas(CanvasAction::SuggestionDown),
            Action::Canvas(CanvasAction::SelectSuggestion),
            Action::Canvas(CanvasAction::ExitSuggestions),
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
fn dispatch_key_event_maps_canvas_keybinding_boundaries_to_focus() {
    let mut read_only = std::collections::HashMap::new();
    read_only.insert("move_down".to_string(), vec!["down".to_string()]);

    let mut editor = FormEditor::new(Provider::new(&["one"]));
    editor.set_keybindings(canvas::CanvasKeyBindings::from_mode_maps(
        &read_only,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    ));

    let outcome: canvas::CanvasKeyDispatchOutcome<(), ()> =
        canvas::dispatch_key_event(&mut editor, key(KeyCode::Down));

    assert!(matches!(
        outcome,
        canvas::CanvasKeyDispatchOutcome::Focus(FocusIntent::ExitCanvasForward)
    ));
    assert_eq!(
        canvas::key_dispatch_status::<Action, _, _>(&outcome),
        Some(TuiPagesStatus::ActionHandled)
    );
}

#[test]
fn dispatch_key_event_maps_named_canvas_undo_redo_actions() {
    let mut read_only = std::collections::HashMap::new();
    read_only.insert("undo".to_string(), vec!["u".to_string()]);
    read_only.insert("redo".to_string(), vec!["ctrl+r".to_string()]);

    let mut editor = FormEditor::new(Provider::new(&[""]));
    editor.set_keybindings(canvas::CanvasKeyBindings::from_mode_maps(
        &read_only,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
    ));

    editor.execute(CanvasAction::EnterEditMode);
    editor.execute(CanvasAction::InsertChar('a'));
    editor.execute(CanvasAction::ExitEditMode);

    let undo: canvas::CanvasKeyDispatchOutcome<(), ()> =
        canvas::dispatch_key_event(&mut editor, key(KeyCode::Char('u')));
    assert!(matches!(
        undo,
        canvas::CanvasKeyDispatchOutcome::Consumed(_)
    ));
    assert_eq!(editor.data_provider().field_value(0), "");

    let redo: canvas::CanvasKeyDispatchOutcome<(), ()> = canvas::dispatch_key_event(
        &mut editor,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
    );
    assert!(matches!(
        redo,
        canvas::CanvasKeyDispatchOutcome::Consumed(_)
    ));
    assert_eq!(editor.data_provider().field_value(0), "a");
}

#[test]
fn boundary_helper_maps_exits_to_intents() {
    let fwd: FocusIntent = canvas::focus_intent_for_boundary(canvas::BoundaryExit::Bottom);
    let back: FocusIntent = canvas::focus_intent_for_boundary(canvas::BoundaryExit::Top);
    assert_eq!(fwd, FocusIntent::ExitCanvasForward);
    assert_eq!(back, FocusIntent::ExitCanvasBackward);
}

#[test]
fn canvas_focus_coexists_with_buttons_sections_modal_and_internal_targets() {
    let mut app = TuiPages::<View, Action, State>::builder(View::Form)
        .page_fn(mixed_focus_page)
        .handler(Handler)
        .bind(tui_pages::modes::NORMAL, "tab", Action::FocusNext)
        .bind(tui_pages::modes::NORMAL, "shift+tab", Action::FocusPrev)
        .bind(tui_pages::modes::NORMAL, "enter", Action::Activate)
        .bind(tui_pages::modes::GLOBAL, "ctrl+o", Action::ShowModal)
        .bind(tui_pages::modes::GLOBAL, "esc", Action::ClearOverlay)
        .build();
    let mut state = State::default();
    app.refresh_page(&state);

    assert_eq!(app.focus.current(), Some(FocusTarget::CanvasField(0)));
    app.apply_effect(TuiEffect::Focus(FocusIntent::ExitCanvasForward), &state);
    assert_eq!(app.focus.current(), Some(FocusTarget::Button(0)));
    app.handle_key(key(KeyCode::Tab), &mut state).unwrap();
    assert_eq!(app.focus.current(), Some(FocusTarget::Section(7)));
    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();
    assert_eq!(
        app.focus.current(),
        Some(FocusTarget::SectionItem {
            section: 7,
            item: 0
        })
    );
    app.handle_key(key(KeyCode::Tab), &mut state).unwrap();
    assert_eq!(
        app.focus.current(),
        Some(FocusTarget::SectionItem {
            section: 7,
            item: 1
        })
    );
    app.handle_key(key(KeyCode::Tab), &mut state).unwrap();
    assert_eq!(app.focus.current(), Some(FocusTarget::CanvasField(1)));

    app.handle_key(
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();
    assert_eq!(app.focus.current(), Some(FocusTarget::ModalItem(0)));
    app.handle_key(key(KeyCode::Esc), &mut state).unwrap();
    assert_eq!(app.focus.current(), Some(FocusTarget::CanvasField(1)));

    app.apply_effect(TuiEffect::Navigate(View::Second), &state);
    app.apply_effect(TuiEffect::PreviousBuffer, &state);
    app.apply_effect(TuiEffect::NextBuffer, &state);
    app.apply_effect(
        TuiEffect::SplitPane(tui_pages::PaneSplit::Horizontal),
        &state,
    );
    app.apply_effect(TuiEffect::NextPane, &state);
    assert!(matches!(
        app.focus.current(),
        Some(FocusTarget::CanvasField(_))
    ));

    assert!(
        app.focus
            .targets()
            .contains(&FocusTarget::InternalCanvasField(99))
    );
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

    assert_eq!(canvas::mode_for_app_mode(AppMode::Ins), modes::INSERT);
    assert_eq!(canvas::mode_for_app_mode(AppMode::Sel), modes::SELECT);
    assert_eq!(canvas::mode_for_app_mode(AppMode::Nor), modes::NORMAL);
    assert!(canvas::accepts_text_input(AppMode::Ins));
    assert!(!canvas::accepts_text_input(AppMode::Nor));
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

#[test]
fn suggestions_filter_select_escape_and_field_transition_work() {
    #[derive(Debug)]
    struct SuggestingProvider {
        values: Vec<String>,
    }

    impl DataProvider for SuggestingProvider {
        fn field_count(&self) -> usize {
            self.values.len()
        }

        fn field_name(&self, index: usize) -> &str {
            match index {
                0 => "tag",
                _ => "notes",
            }
        }

        fn field_value(&self, index: usize) -> &str {
            &self.values[index]
        }

        fn set_field_value(&mut self, index: usize, value: String) {
            self.values[index] = value;
        }

        fn supports_suggestions(&self, field_index: usize) -> bool {
            field_index == 0
        }

        fn suggestion_trigger(&self, field_index: usize) -> canvas::SuggestionTrigger {
            if field_index == 0 {
                canvas::SuggestionTrigger::WhenFieldStarts
            } else {
                canvas::SuggestionTrigger::None
            }
        }

        fn fetch_suggestions_sync(
            &self,
            _field_index: usize,
            query: &str,
        ) -> Vec<canvas::SuggestionItem> {
            ["alpha", "atom", "beta"]
                .into_iter()
                .filter(|item| item.starts_with(query))
                .map(|item| canvas::SuggestionItem::new(item, item))
                .collect()
        }
    }

    let mut editor = FormEditor::new(SuggestingProvider {
        values: vec!["a".to_string(), String::new()],
    });

    editor.execute(CanvasAction::TriggerSuggestions);
    assert!(editor.is_suggestions_active());
    assert_eq!(editor.suggestions().len(), 2);

    editor.execute(CanvasAction::SuggestionDown);
    editor.execute(CanvasAction::SelectSuggestion);
    assert_eq!(editor.current_text(), "atom");
    assert!(!editor.is_suggestions_active());

    editor.execute(CanvasAction::TriggerSuggestions);
    assert!(editor.is_suggestions_active());
    editor.execute(CanvasAction::ExitSuggestions);
    assert!(!editor.is_suggestions_active());

    editor.execute(CanvasAction::TriggerSuggestions);
    assert!(editor.is_suggestions_active());
    editor.execute(CanvasAction::MoveDown);
    assert!(!editor.is_suggestions_active());
    assert_eq!(editor.current_field(), 1);
}

#[test]
fn validation_blocked_navigation_surfaces_as_action_error() {
    let mut editor = FormEditor::new(Provider::new(&["ab", "next"]));
    editor.set_field_validation(
        0,
        canvas::ValidationConfigBuilder::new()
            .with_character_limits(canvas::CharacterLimits::new_min(3))
            .build(),
    );

    let outcome: CanvasDispatchOutcome<(), ()> =
        canvas::dispatch_action(&mut editor, CanvasAction::MoveDown);

    match outcome {
        CanvasDispatchOutcome::Applied(canvas::ActionResult::Error(message)) => {
            assert!(message.contains("at least 3 characters"));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
    assert_eq!(editor.current_field(), 0);
}

#[test]
fn validation_display_masks_and_character_filters_are_reachable() {
    let mask = canvas::DisplayMask::new("(###) ###-####", '#');
    let config = canvas::ValidationConfigBuilder::new()
        .with_display_mask(mask)
        .with_pattern_filters(canvas::PatternFilters::new().add_filter(
            canvas::PositionFilter::new(
                canvas::PositionRange::From(0),
                canvas::CharacterFilter::Numeric,
            ),
        ))
        .build();
    let mut editor = FormEditor::new(Provider::new(&[""]));
    editor.set_field_validation(0, config);
    editor.enter_edit_mode();

    let _ = editor.insert_char('x');
    assert_eq!(editor.current_text(), "");
    let _ = editor.insert_char('1');
    assert_eq!(editor.current_text(), "1");
    assert_eq!(editor.display_text_for_field(0), "(1");
}

#[test]
fn computed_fields_are_recomputed_and_skipped_by_navigation() {
    struct SumProvider;

    impl canvas::ComputedProvider for SumProvider {
        fn compute_field(&mut self, context: canvas::ComputedContext) -> String {
            let left = context.field_values[0].parse::<i32>().unwrap_or_default();
            let right = context.field_values[1].parse::<i32>().unwrap_or_default();
            (left + right).to_string()
        }

        fn handles_field(&self, field_index: usize) -> bool {
            field_index == 2
        }

        fn field_dependencies(&self, _field_index: usize) -> Vec<usize> {
            vec![0, 1]
        }
    }

    let mut editor = FormEditor::new(Provider::new(&["2", "3", ""]));
    let mut provider = SumProvider;
    editor.register_computed_provider(&provider);
    editor.recompute_all_fields(&mut provider);

    assert_eq!(editor.effective_field_value(2), "5");
    editor.execute(CanvasAction::MoveDown);
    assert_eq!(editor.current_field(), 1);
    editor.execute(CanvasAction::MoveDown);
    assert_eq!(editor.current_field(), 1);
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
    input.enter_edit_mode();
    input.set_cursor_position(3);
    input.set_suggestion_suffix("uments");
    let outcome = input.accept_suggestion_suffix();

    assert_eq!(outcome, TextInputEventOutcome::Handled);
    assert_eq!(input.text(), "documents");
    assert_eq!(input.suggestion_suffix(), None);
}

#[test]
fn text_input_dispatch_maps_submit_and_boundary_exit() {
    let mut input = canvas::TextInputState::<canvas::TextInputProvider>::from_text("hello");

    let submitted: canvas::CanvasTextWidgetOutcome<(), ()> =
        canvas::dispatch_text_input_key(&mut input, key(KeyCode::Enter));
    assert_eq!(submitted, canvas::CanvasTextWidgetOutcome::Submitted);

    let exit: canvas::CanvasTextWidgetOutcome<(), ()> =
        canvas::dispatch_text_input_key(&mut input, key(KeyCode::Down));
    assert_eq!(
        exit,
        canvas::CanvasTextWidgetOutcome::Focus(FocusIntent::ExitCanvasForward)
    );
}

#[test]
fn text_input_dispatch_handles_typing_delete_and_tab_suffix() {
    let mut input = canvas::TextInputState::<canvas::TextInputProvider>::from_text("ab");
    input.enter_edit_mode();
    input.set_cursor_position(2);

    assert_eq!(
        canvas::dispatch_text_input_key::<_, (), ()>(&mut input, key(KeyCode::Char('c'))),
        canvas::CanvasTextWidgetOutcome::Handled
    );
    assert_eq!(input.text(), "abc");

    let _ = canvas::dispatch_text_input_key::<_, (), ()>(&mut input, key(KeyCode::Backspace));
    assert_eq!(input.text(), "ab");

    input.set_suggestion_suffix("cd");
    let _ = canvas::dispatch_text_input_key::<_, (), ()>(&mut input, key(KeyCode::Tab));
    assert_eq!(input.text(), "abcd");
}

#[test]
fn textarea_dispatch_maps_vertical_boundaries_to_focus() {
    let mut textarea = canvas::TextAreaState::<canvas::TextAreaProvider>::from_text("first\nlast");

    let top: canvas::CanvasTextWidgetOutcome<(), ()> =
        canvas::dispatch_text_area_key(&mut textarea, key(KeyCode::Up));
    assert_eq!(
        top,
        canvas::CanvasTextWidgetOutcome::Focus(FocusIntent::ExitCanvasBackward)
    );

    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Down));
    let bottom: canvas::CanvasTextWidgetOutcome<(), ()> =
        canvas::dispatch_text_area_key(&mut textarea, key(KeyCode::Down));
    assert_eq!(
        bottom,
        canvas::CanvasTextWidgetOutcome::Focus(FocusIntent::ExitCanvasForward)
    );
}

#[test]
fn textarea_dispatch_handles_multiline_editing_paste_and_movement() {
    let mut textarea = canvas::TextAreaState::<canvas::TextAreaProvider>::from_text("");

    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Char('a')));
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Enter));
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Char('b')));
    assert_eq!(textarea.text(), "a\nb");

    assert_eq!(textarea.paste("\nc"), canvas::TextAreaEventOutcome::Handled);
    assert_eq!(textarea.text(), "a\nb\nc");
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Up));
    assert_eq!(textarea.current_field(), 1);
}

#[test]
fn textarea_dispatch_uses_default_commandline_when_enabled() {
    let mut textarea = canvas::TextAreaState::<canvas::TextAreaProvider>::from_text("needle\nhay");
    textarea.use_default_commandline();

    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Char('/')));
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Char('h')));
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Char('a')));
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Char('y')));
    let _ = canvas::dispatch_text_area_key::<_, (), ()>(&mut textarea, key(KeyCode::Enter));

    assert_eq!(textarea.search_query(), Some("hay"));
    assert_eq!(
        textarea.active_search_match(),
        Some(canvas::TextAreaSearchMatch {
            line: 1,
            start: 0,
            end: 3,
        })
    );
}

// --- Builder widgets ----------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum WidgetAction {
    Quit,
}

struct WidgetState {
    editor: FormEditor<Provider>,
    textarea: canvas::TextAreaState<canvas::TextAreaProvider>,
    textinput: canvas::TextInputState<canvas::TextInputProvider>,
    textarea_entered: bool,
    textinput_entered: bool,
}

impl Default for WidgetState {
    fn default() -> Self {
        Self {
            editor: FormEditor::new(Provider::new(&[""])),
            textarea: canvas::TextAreaState::from_text(""),
            textinput: canvas::TextInputState::from_text(""),
            textarea_entered: false,
            textinput_entered: false,
        }
    }
}

impl canvas::CanvasWidgetState for WidgetState {
    fn canvas_form_editor_ref(&self, id: usize) -> Option<&dyn canvas::CanvasFormEditorHost> {
        match id {
            0 => Some(&self.editor),
            _ => None,
        }
    }

    fn canvas_form_editor(&mut self, id: usize) -> Option<&mut dyn canvas::CanvasFormEditorHost> {
        match id {
            0 => Some(&mut self.editor),
            _ => None,
        }
    }

    fn canvas_textarea_ref(&self, focus_index: usize) -> Option<&dyn canvas::CanvasTextAreaHost> {
        match focus_index {
            0 => Some(&self.textarea),
            _ => None,
        }
    }

    fn canvas_textarea(
        &mut self,
        focus_index: usize,
    ) -> Option<&mut dyn canvas::CanvasTextAreaHost> {
        match focus_index {
            0 => Some(&mut self.textarea),
            _ => None,
        }
    }

    fn canvas_textarea_entered_ref(&self, focus_index: usize) -> Option<&bool> {
        match focus_index {
            0 => Some(&self.textarea_entered),
            _ => None,
        }
    }

    fn canvas_textarea_entered(&mut self, focus_index: usize) -> Option<&mut bool> {
        match focus_index {
            0 => Some(&mut self.textarea_entered),
            _ => None,
        }
    }

    fn canvas_textinput_ref(&self, focus_index: usize) -> Option<&dyn canvas::CanvasTextInputHost> {
        match focus_index {
            0 => Some(&self.textinput),
            _ => None,
        }
    }

    fn canvas_textinput(
        &mut self,
        focus_index: usize,
    ) -> Option<&mut dyn canvas::CanvasTextInputHost> {
        match focus_index {
            0 => Some(&mut self.textinput),
            _ => None,
        }
    }

    fn canvas_textinput_entered_ref(&self, focus_index: usize) -> Option<&bool> {
        match focus_index {
            0 => Some(&self.textinput_entered),
            _ => None,
        }
    }

    fn canvas_textinput_entered(&mut self, focus_index: usize) -> Option<&mut bool> {
        match focus_index {
            0 => Some(&mut self.textinput_entered),
            _ => None,
        }
    }

    fn canvas_textinput_suggestion_suffix(
        &mut self,
        focus_index: usize,
        text: &str,
    ) -> Option<String> {
        match (focus_index, text) {
            (0, "a") => Some("dmin".to_string()),
            _ => None,
        }
    }
}

struct WidgetHandler;

impl TuiActionHandler<View, WidgetAction, WidgetState> for WidgetHandler {
    type Error = std::convert::Infallible;

    fn handle_action(
        &mut self,
        action: WidgetAction,
        _ctx: ActionContext<View>,
        _state: &mut WidgetState,
    ) -> Result<ActionOutcome<View>, Self::Error> {
        Ok(match action {
            WidgetAction::Quit => ActionOutcome::effect(TuiEffect::Quit),
        })
    }
}

fn widget_page(_v: &View, _s: &WidgetState, _f: Option<&FocusTarget>) -> PageSpec {
    PageSpec::new().focus(PageFocusBuilder::new().canvas_field(0).button(0))
}

#[test]
fn canvas_form_editor_builder_dispatches_without_canvas_actions_in_app_action() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_form_editor(0)
        .bind(tui_pages::modes::GLOBAL, "ctrl+c", WidgetAction::Quit)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
        &mut state,
    )
    .unwrap();

    assert_eq!(state.editor.data_provider().field_value(0), "A");
}

#[test]
fn canvas_insert_mode_owns_text_even_without_page_mode_sync() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_form_editor(0)
        .bind(tui_pages::modes::GLOBAL, "j", WidgetAction::Quit)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    let output = app.handle_key(key(KeyCode::Char('j')), &mut state).unwrap();

    assert!(!output.quit_requested);
    assert_eq!(state.editor.data_provider().field_value(0), "j");
}

#[test]
fn canvas_form_editor_receives_bracketed_paste() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_form_editor(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    // The canvas field is focused by default, so a paste lands in the editor
    // in a single insert (no per-character key events needed).
    let out = app.handle_paste("pasted text", &mut state).unwrap();

    assert_eq!(out.status, TuiPagesStatus::TextHandled);
    assert_eq!(state.editor.data_provider().field_value(0), "pasted text");
}

#[test]
fn runtime_rebind_canvas_reinstalls_existing_form_editor_bindings() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_form_editor(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
        &mut state,
    )
    .unwrap();
    app.handle_key(key(KeyCode::Esc), &mut state).unwrap();
    assert_eq!(state.editor.data_provider().field_value(0), "A");

    app.rebind_canvas(AppMode::Nor, "undo", vec!["U".to_string()])
        .unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT),
        &mut state,
    )
    .unwrap();

    assert_eq!(state.editor.data_provider().field_value(0), "");
}

#[test]
fn canvas_config_profile_preserves_exit_suggestions_binding() {
    // Regression for the lossy pipeline rebuild: a runtime config change used to
    // reproject the canvas bindings through a path that could not represent
    // `ExitSuggestions` (the canvas keybinding enum lacked the variant), so
    // Ctrl+G silently stopped exiting the suggestion dropdown after any
    // `apply_keybindings_toml` / `rebind_canvas`. The profile that now drives
    // every widget must carry that binding straight from the preset.
    use canvas::{CanvasKeyAction, KeyStroke};

    let config =
        tui_pages::keybindings::KeybindingConfig::from_toml("[canvas]\npreset = \"vim\"\n")
            .unwrap();
    let profile = config.canvas_profile().unwrap();

    let ctrl_g = [KeyStroke {
        code: KeyCode::Char('g'),
        modifiers: KeyModifiers::CONTROL,
    }];
    assert_eq!(
        profile.current().lookup_action(AppMode::Ins, &ctrl_g).0,
        Some(&CanvasKeyAction::ExitSuggestions),
    );
}

#[test]
fn handle_paste_is_a_noop_without_a_focused_canvas_widget() {
    // No canvas hook registered, so there is nothing to receive the paste.
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    let out = app.handle_paste("ignored", &mut state).unwrap();

    assert_eq!(out.status, TuiPagesStatus::Cancelled);
    assert_eq!(state.editor.data_provider().field_value(0), "");
}

#[test]
fn canvas_form_editor_builder_hands_off_focus_at_boundaries() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_form_editor(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    let output = app.handle_key(key(KeyCode::Down), &mut state).unwrap();

    assert_eq!(output.status, TuiPagesStatus::ActionHandled);
    assert_eq!(app.focus.current(), Some(FocusTarget::Button(0)));
}

#[test]
fn canvas_textarea_widget_builder_owns_enter_edit_and_exit_flow() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_textarea_widget(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();
    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
        &mut state,
    )
    .unwrap();
    app.handle_key(key(KeyCode::Esc), &mut state).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
        &mut state,
    )
    .unwrap();

    assert_eq!(state.textarea.text(), "A");
    assert!(!state.textarea_entered);
    assert_eq!(app.focus.current(), Some(FocusTarget::Button(0)));
}

#[test]
fn canvas_textarea_widget_builder_uses_default_commandline() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_textarea_widget(0)
        .build();
    let mut state = WidgetState::default();
    state.textarea = canvas::TextAreaState::from_text("first\nsecond");
    state.textarea.use_default_commandline();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();
    app.handle_key(key(KeyCode::Char(':')), &mut state).unwrap();
    for ch in "set number".chars() {
        app.handle_key(key(KeyCode::Char(ch)), &mut state).unwrap();
    }
    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();

    assert_eq!(
        state.textarea.line_number_mode(),
        canvas::TextAreaLineNumberMode::Absolute
    );
}

#[test]
fn helix_textarea_widget_uses_keybinding_engine_without_commandline() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_textarea_widget_with_preset(0, canvas::BuiltinCanvasKeybindingPreset::Helix)
        .build();
    let mut state = WidgetState::default();
    state.textarea = canvas::TextAreaState::from_text("abc");
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();
    assert_eq!(state.textarea.mode(), AppMode::Nor);

    app.handle_key(key(KeyCode::Char('x')), &mut state).unwrap();

    assert_ne!(state.textarea.text(), "abcx");
    assert_eq!(state.textarea.mode(), AppMode::Nor);

    app.handle_key(key(KeyCode::Char('v')), &mut state).unwrap();
    assert_eq!(state.textarea.mode(), AppMode::Sel);

    app.handle_key(key(KeyCode::Esc), &mut state).unwrap();

    assert!(state.textarea_entered);
}

#[test]
fn canvas_textarea_widget_builder_treats_unentered_widget_as_one_focus_stop() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_textarea_widget(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Char('j')), &mut state).unwrap();

    assert_eq!(app.focus.current(), Some(FocusTarget::Button(0)));
    assert!(!state.textarea_entered);
}

#[test]
fn canvas_textinput_widget_builder_dispatches_raw_text_and_submit_focus() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_textinput_widget(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();
    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    app.handle_key(
        KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT),
        &mut state,
    )
    .unwrap();
    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();

    assert_eq!(state.textinput.text(), "A");
    assert!(!state.textinput_entered);
    assert_eq!(app.focus.current(), Some(FocusTarget::Button(0)));
}

#[test]
fn canvas_textinput_widget_builder_updates_inline_suggestion_suffix() {
    let mut app = TuiPages::<View, WidgetAction, WidgetState>::builder(View::Form)
        .page_fn(widget_page)
        .handler(WidgetHandler)
        .canvas_textinput_widget(0)
        .build();
    let mut state = WidgetState::default();
    app.refresh_page(&state);

    app.handle_key(key(KeyCode::Enter), &mut state).unwrap();
    app.handle_key(key(KeyCode::Char('i')), &mut state).unwrap();
    app.handle_key(key(KeyCode::Char('a')), &mut state).unwrap();

    assert_eq!(state.textinput.text(), "a");
    assert_eq!(state.textinput.suggestion_suffix(), Some("dmin"));

    app.handle_key(key(KeyCode::Tab), &mut state).unwrap();

    assert_eq!(state.textinput.text(), "admin");
    assert_eq!(state.textinput.suggestion_suffix(), None);
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
    let _opts = canvas::CanvasDisplayOptions {
        max_label_width: 18,
        max_input_width: Some(40),
        row_input_width: Some(|row, available| match row {
            0 => 12.min(available),
            1 => 40.min(available),
            _ => available,
        }),
        ..Default::default()
    };
    let _overflow: Option<canvas::OverflowMode> = None;
    // `render_canvas_default` draws into a ratatui `Frame`; we only prove the
    // path resolves (rendering needs a live terminal/frame to call).
    let _render = canvas::render_canvas_default::<Provider>;
    let _render_with_suggestions = canvas::render_canvas_with_suggestions_default::<Provider>;
    let _render_with_suggestions_options =
        canvas::render_canvas_with_suggestions_default_options::<Provider>;

    // Cursor style.
    let _cursor: Option<canvas::CursorManager> = None;
    let _cursor_mode = canvas::update_cursor_style_for_mode;

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
    let _: Option<canvas::TextAreaCommandLineState> = None;
    let _line_numbers = canvas::TextAreaLineNumberMode::Absolute;
    let _search_match = canvas::TextAreaSearchMatch {
        line: 0,
        start: 0,
        end: 1,
    };
    let _: Option<canvas::TextInputProvider> = None;

    // Command line.
    let _commandline = canvas::CommandLineState::new();
    let _commandline_widget = canvas::CommandLine::default();
    let _commandline_mode = canvas::CommandLineMode::Command;
    let _commandline_submit = canvas::CommandLineSubmit::Command("write".to_string());
    let _parsed = canvas::parse_command_line("set number");
    let _args = canvas::parse_command_args("set number");
    let _registry = canvas::CommandLineRegistry::new();
    let _command = canvas::CommandLineCommand::new("write");
    let _: Option<canvas::CommandLineEventOutcome> = None;
    let _: Option<canvas::CommandLinePlacement> = None;
    let _: Option<canvas::CommandLineDispatchError> = None;
    let _: Option<canvas::CommandLineParseError> = None;
    let _: Option<canvas::CommandLineParsedCommand> = None;
    let _: Option<canvas::CommandLineCommandInvocation> = None;
    let _: Option<canvas::CommandLineRegistrationError> = None;

    // Keybindings + host handoff. `CanvasKeyBindings` is built from mode->binding maps.
    let empty = std::collections::HashMap::new();
    let _keybindings = canvas::CanvasKeyBindings::from_mode_maps(&empty, &empty, &empty);
    let _builtin_bindings =
        canvas::default_builtin_action_bindings(canvas::BuiltinCanvasKeybindingPreset::Vim);
    let _helix_bindings = canvas::default_helix_action_bindings();
    let _emacs_bindings = canvas::default_emacs_action_bindings();
    let _: Option<canvas::CanvasActionKeyBinding> = None;
    let _: Option<canvas::KeyEventOutcome> = None;
    let _: Option<canvas::CanvasKeyAction> = None;
    let _: Option<canvas::HostKeyEventOutcome> = None;
    let _: Option<canvas::CanvasKeyDispatchOutcome> = None;
    let _: Option<canvas::CanvasTextWidgetOutcome> = None;

    // Crossterm session helpers.
    let _: Option<canvas::CrosstermInputOptions> = None;

    // Touch a value so the bindings are not all dead.
    assert!(_action.is_editing_action());
    assert!(_result.is_success());
    let _ = (
        _theme,
        _opts,
        _trigger,
        _query,
        _keybindings,
        _builtin_bindings,
        _helix_bindings,
        _emacs_bindings,
        _render,
        _render_with_suggestions,
        _render_with_suggestions_options,
        _cursor_mode,
        _commandline,
        _commandline_widget,
        _commandline_mode,
        _commandline_submit,
        _parsed,
        _args,
        _registry,
        _command,
        _line_numbers,
        _search_match,
    );
}

#[test]
fn canvas_suggestions_renderer_honors_per_row_input_width_options() {
    use ratatui::{Terminal, backend::TestBackend, layout::Rect};

    fn row_width(row: usize, available: u16) -> u16 {
        match row {
            0 => 8.min(available),
            1 => 31.min(available),
            _ => available,
        }
    }

    let mut editor = FormEditor::new(Provider::new(&["short", "wide"]));
    let _ = editor.execute(CanvasAction::MoveDown);

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut active_rect = None;

    terminal
        .draw(|frame| {
            active_rect = canvas::render_canvas_with_suggestions_default_options(
                frame,
                frame.area(),
                Rect::new(0, 0, 80, 10),
                &editor,
                canvas::CanvasDisplayOptions {
                    max_label_width: 10,
                    max_input_width: None,
                    row_input_width: Some(row_width),
                    ..Default::default()
                },
            );
        })
        .unwrap();

    assert_eq!(active_rect.map(|rect| rect.width), Some(31));
}
