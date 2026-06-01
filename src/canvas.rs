//! Integration helpers for using the `canvas` crate with `tui-pages`.
//!
//! Enable the `canvas` feature and make your application action type implement
//! `From<CanvasAction>`. Then `.canvas_defaults()` on the builder installs the
//! standard `FormEditor` keymaps and typed-character action routing, while
//! [`PageSpec::canvas_editor`] keeps the active mode stack in sync with the
//! editor.

use crate::focus::FocusIntent;
use crate::input::{KeyChord, KeyMap};
use crate::runtime::{modes, ModeId, PageSpec, TuiPagesBuilder, TuiPagesStatus};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, Frame};

// The `canvas` feature enables full canvas support — every surface below is
// available unconditionally. We do not split canvas into sub-features.

// --- Base surface ---
pub use ::canvas::{ActionResult, AppMode, CanvasAction, DataProvider, EditorState, FormEditor};
pub use ::canvas::integration::focus_handoff::{
    execute_action_for_host, execute_action_for_host_with_options, BoundaryExit, HostActionOutcome,
};

// --- Keymap-driven host handoff ---
pub use ::canvas::integration::focus_handoff::{
    boundary_from_key_outcome, handle_key_event_for_host, key_outcome_for_vertical_navigation,
    map_key_event_outcome_for_host, HostKeyEventOutcome,
};
pub use ::canvas::{CanvasKeyMap, KeyEventOutcome};
pub use ::canvas::keymap::{CanvasKeyAction, KeyStroke};

// --- Cursor style ---
pub use ::canvas::CursorManager;

// --- Suggestions ---
pub use ::canvas::{
    render_suggestions_dropdown, SuggestionItem, SuggestionQuery, SuggestionTrigger,
};

// --- Validation ---
pub use ::canvas::{
    AppliedValidation, CharacterFilter, CharacterLimits, CustomFormatter, DefaultPositionMapper,
    DisplayMask, FormattingResult, PatternFilters, PositionFilter, PositionMapper, PositionRange,
    ValidationConfig, ValidationConfigBuilder, ValidationError, ValidationResult, ValidationRule,
    ValidationSet, ValidationSettings, ValidationState, ValidationSummary,
};

// --- Computed fields ---
pub use ::canvas::{ComputedContext, ComputedProvider, ComputedState};

// --- GUI: renderers, themes, display options ---
pub use ::canvas::{
    render_canvas, render_canvas_default, render_canvas_with_options, CanvasDisplayOptions,
    CanvasTheme, DefaultCanvasTheme, FormInputEventOutcome, OverflowMode,
};

// Crossterm terminal-input session helpers (raw mode, bracketed paste, mouse
// capture) — the canvas-side complement to [`crate::terminal`] for apps wiring
// up text widgets that need paste support.
pub use ::canvas::integration::crossterm_input::{
    CrosstermInputGuard, CrosstermInputOptions, CrosstermInputSession,
};

// --- Text area ---
pub use ::canvas::{
    TextArea, TextAreaDataProvider, TextAreaEditor, TextAreaProvider, TextAreaState,
};
pub use ::canvas::textarea::{TextAreaEventOutcome, TextOverflowMode};

// --- Text input ---
pub use ::canvas::{
    TextInput, TextInputDataProvider, TextInputEditor, TextInputEventOutcome, TextInputProvider,
    TextInputState,
};

#[derive(Debug, Clone)]
pub enum CanvasDispatchOutcome<O = (), M = ()> {
    Applied(::canvas::ActionResult),
    Focus(FocusIntent<O, M>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasKeyDispatchOutcome<O = (), M = ()> {
    Consumed(Option<String>),
    PendingSequence,
    NotHandled,
    Focus(FocusIntent<O, M>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasTextWidgetOutcome<O = (), M = ()> {
    Handled,
    Submitted,
    NotHandled,
    Focus(FocusIntent<O, M>),
}

impl<O, M> CanvasDispatchOutcome<O, M> {
    pub fn into_focus_intent(self) -> Option<FocusIntent<O, M>> {
        match self {
            CanvasDispatchOutcome::Applied(_) => None,
            CanvasDispatchOutcome::Focus(intent) => Some(intent),
        }
    }
}

impl<O, M> CanvasKeyDispatchOutcome<O, M> {
    pub fn into_focus_intent(self) -> Option<FocusIntent<O, M>> {
        match self {
            CanvasKeyDispatchOutcome::Focus(intent) => Some(intent),
            _ => None,
        }
    }
}

impl<O, M> CanvasTextWidgetOutcome<O, M> {
    pub fn into_focus_intent(self) -> Option<FocusIntent<O, M>> {
        match self {
            CanvasTextWidgetOutcome::Focus(intent) => Some(intent),
            _ => None,
        }
    }
}

pub fn mode_for_app_mode(mode: AppMode) -> ModeId {
    match mode {
        AppMode::Edit => modes::INSERT,
        AppMode::Highlight => modes::SELECT,
        AppMode::Command => modes::COMMAND,
        AppMode::General => modes::GENERAL,
        AppMode::ReadOnly => modes::NORMAL,
    }
}

pub fn modes_for_app_mode(mode: AppMode) -> Vec<ModeId> {
    match mode {
        AppMode::Command => vec![modes::COMMAND],
        AppMode::General => vec![modes::GENERAL, modes::GLOBAL],
        mode => vec![mode_for_app_mode(mode), modes::COMMON, modes::GLOBAL],
    }
}

pub fn accepts_text_input(mode: AppMode) -> bool {
    matches!(mode, AppMode::Edit | AppMode::Command)
}

pub fn text_chord_to_canvas_action(chord: KeyChord) -> Option<CanvasAction> {
    let is_plain_char =
        chord.modifiers.is_empty() || chord.modifiers == KeyModifiers::SHIFT;
    match chord.code {
        KeyCode::Char(c) if is_plain_char => Some(CanvasAction::InsertChar(c)),
        _ => None,
    }
}

pub fn text_chord_to_action<A>(chord: KeyChord) -> Option<A>
where
    A: From<CanvasAction>,
{
    text_chord_to_canvas_action(chord).map(A::from)
}

pub fn focus_intent_for_boundary<O, M>(boundary: BoundaryExit) -> FocusIntent<O, M> {
    match boundary {
        BoundaryExit::Top => FocusIntent::ExitCanvasBackward,
        BoundaryExit::Bottom => FocusIntent::ExitCanvasForward,
    }
}

pub fn dispatch_action<D, O, M>(
    editor: &mut FormEditor<D>,
    action: CanvasAction,
) -> CanvasDispatchOutcome<O, M>
where
    D: DataProvider,
{
    let before_field = editor.current_field();
    let at_boundary = action_boundary(editor, &action).is_some();
    match execute_action_for_host(editor, action) {
        HostActionOutcome::Applied(result) => {
            CanvasDispatchOutcome::Applied(validation_aware_action_result(
                editor,
                before_field,
                at_boundary,
                result,
            ))
        }
        HostActionOutcome::ExitCanvas(boundary) => {
            CanvasDispatchOutcome::Focus(focus_intent_for_boundary(boundary))
        }
    }
}

pub fn render_canvas_with_suggestions<T, D>(
    frame: &mut Frame,
    frame_area: Rect,
    canvas_area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
) -> Option<Rect>
where
    T: CanvasTheme,
    D: DataProvider,
{
    let input_rect = render_canvas(frame, canvas_area, editor, theme);
    if let Some(input_rect) = input_rect {
        render_suggestions_dropdown(frame, frame_area, input_rect, theme, editor);
    }
    input_rect
}

pub fn render_canvas_with_suggestions_default<D>(
    frame: &mut Frame,
    frame_area: Rect,
    canvas_area: Rect,
    editor: &FormEditor<D>,
) -> Option<Rect>
where
    D: DataProvider,
{
    let theme = DefaultCanvasTheme;
    render_canvas_with_suggestions(frame, frame_area, canvas_area, editor, &theme)
}

pub fn update_cursor_style_for_mode(mode: AppMode) -> std::io::Result<()> {
    CursorManager::update_for_mode(mode)
}

pub fn update_cursor_style_for_editor<D>(editor: &FormEditor<D>) -> std::io::Result<()>
where
    D: DataProvider,
{
    update_cursor_style_for_mode(editor.mode())
}

pub fn dispatch_key_event<D, O, M>(
    editor: &mut FormEditor<D>,
    event: KeyEvent,
) -> CanvasKeyDispatchOutcome<O, M>
where
    D: DataProvider,
{
    let before_field = editor.current_field();
    let before_boundary = key_boundary(editor, &event);
    let outcome = handle_key_event_for_host(editor, event);
    host_key_event_outcome(validation_aware_key_event_outcome(
        editor,
        before_field,
        before_boundary,
        outcome,
    ))
}

pub fn host_key_event_outcome<O, M>(
    outcome: HostKeyEventOutcome,
) -> CanvasKeyDispatchOutcome<O, M> {
    match outcome {
        HostKeyEventOutcome::Consumed(message) => CanvasKeyDispatchOutcome::Consumed(message),
        HostKeyEventOutcome::PendingSequence => CanvasKeyDispatchOutcome::PendingSequence,
        HostKeyEventOutcome::NotHandled => CanvasKeyDispatchOutcome::NotHandled,
        HostKeyEventOutcome::ExitCanvas(boundary) => {
            CanvasKeyDispatchOutcome::Focus(focus_intent_for_boundary(boundary))
        }
    }
}

fn validation_aware_action_result<D>(
    editor: &FormEditor<D>,
    before_field: usize,
    at_boundary: bool,
    result: ActionResult,
) -> ActionResult
where
    D: DataProvider,
{
    if editor.current_field() == before_field && !at_boundary {
        if let Some(reason) = editor.last_switch_block() {
            return ActionResult::Error(reason.to_string());
        }
    }
    result
}

fn validation_aware_key_event_outcome<D>(
    editor: &FormEditor<D>,
    before_field: usize,
    before_boundary: Option<BoundaryExit>,
    outcome: HostKeyEventOutcome,
) -> HostKeyEventOutcome
where
    D: DataProvider,
{
    if matches!(outcome, HostKeyEventOutcome::ExitCanvas(_))
        && editor.current_field() == before_field
        && before_boundary.is_none()
    {
        if let Some(reason) = editor.last_switch_block() {
            return HostKeyEventOutcome::Consumed(Some(reason.to_string()));
        }
    }
    outcome
}

fn action_boundary<D>(editor: &FormEditor<D>, action: &CanvasAction) -> Option<BoundaryExit>
where
    D: DataProvider,
{
    match action {
        CanvasAction::MoveUp | CanvasAction::PrevField if editor.current_field() == 0 => {
            Some(BoundaryExit::Top)
        }
        CanvasAction::MoveDown | CanvasAction::NextField
            if editor.current_field()
                >= editor.data_provider().field_count().saturating_sub(1) =>
        {
            Some(BoundaryExit::Bottom)
        }
        _ => None,
    }
}

fn key_boundary<D>(editor: &FormEditor<D>, event: &KeyEvent) -> Option<BoundaryExit>
where
    D: DataProvider,
{
    match event.code {
        KeyCode::Up | KeyCode::BackTab if editor.current_field() == 0 => Some(BoundaryExit::Top),
        KeyCode::Down | KeyCode::Tab
            if editor.current_field()
                >= editor.data_provider().field_count().saturating_sub(1) =>
        {
            Some(BoundaryExit::Bottom)
        }
        _ => None,
    }
}

pub fn key_dispatch_status<A, O, M>(
    outcome: &CanvasKeyDispatchOutcome<O, M>,
) -> Option<TuiPagesStatus<A>> {
    match outcome {
        CanvasKeyDispatchOutcome::Consumed(_) | CanvasKeyDispatchOutcome::Focus(_) => {
            Some(TuiPagesStatus::ActionHandled)
        }
        CanvasKeyDispatchOutcome::PendingSequence => Some(TuiPagesStatus::Waiting(Vec::new())),
        CanvasKeyDispatchOutcome::NotHandled => None,
    }
}

pub fn dispatch_text_input_key<P, O, M>(
    input: &mut TextInputState<P>,
    event: KeyEvent,
) -> CanvasTextWidgetOutcome<O, M>
where
    P: TextInputDataProvider,
{
    let boundary = text_input_boundary_for_key(&event);
    match input.input(event) {
        TextInputEventOutcome::Handled => CanvasTextWidgetOutcome::Handled,
        TextInputEventOutcome::Submitted => CanvasTextWidgetOutcome::Submitted,
        TextInputEventOutcome::Ignored => boundary
            .map(|boundary| CanvasTextWidgetOutcome::Focus(focus_intent_for_boundary(boundary)))
            .unwrap_or(CanvasTextWidgetOutcome::NotHandled),
    }
}

pub fn dispatch_text_area_key<P, O, M>(
    textarea: &mut TextAreaState<P>,
    event: KeyEvent,
) -> CanvasTextWidgetOutcome<O, M>
where
    P: TextAreaDataProvider,
{
    if let Some(boundary) = text_area_boundary_for_key(textarea, &event) {
        return CanvasTextWidgetOutcome::Focus(focus_intent_for_boundary(boundary));
    }

    match textarea.input(event) {
        TextAreaEventOutcome::Handled => CanvasTextWidgetOutcome::Handled,
        TextAreaEventOutcome::Ignored => CanvasTextWidgetOutcome::NotHandled,
    }
}

pub fn text_input_boundary_for_key(event: &KeyEvent) -> Option<BoundaryExit> {
    if event.kind != KeyEventKind::Press {
        return None;
    }

    match (event.code, event.modifiers) {
        (KeyCode::Up | KeyCode::BackTab, _) => Some(BoundaryExit::Top),
        (KeyCode::Down, _) => Some(BoundaryExit::Bottom),
        (KeyCode::Tab, modifiers) if modifiers.is_empty() => Some(BoundaryExit::Bottom),
        _ => None,
    }
}

pub fn text_area_boundary_for_key<P>(
    textarea: &TextAreaState<P>,
    event: &KeyEvent,
) -> Option<BoundaryExit>
where
    P: TextAreaDataProvider,
{
    if event.kind != KeyEventKind::Press {
        return None;
    }

    let current = textarea.current_field();
    let last = textarea.data_provider().field_count().saturating_sub(1);

    match event.code {
        KeyCode::Up | KeyCode::BackTab if current == 0 => Some(BoundaryExit::Top),
        KeyCode::Down if current >= last => Some(BoundaryExit::Bottom),
        _ => None,
    }
}

pub fn bind_default_keymaps<A>(
    normal: &mut KeyMap<A>,
    insert: &mut KeyMap<A>,
    select: &mut KeyMap<A>,
)
where
    A: From<CanvasAction>,
{
    bind_normal_defaults(normal);
    bind_insert_defaults(insert);
    bind_select_defaults(select);
}

pub fn bind_normal_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_key(map, KeyCode::Up, CanvasAction::MoveUp);
    bind_key(map, KeyCode::Down, CanvasAction::MoveDown);
    bind_key(map, KeyCode::Left, CanvasAction::MoveLeft);
    bind_key(map, KeyCode::Right, CanvasAction::MoveRight);
    bind_char(map, 'k', CanvasAction::MoveUp);
    bind_char(map, 'j', CanvasAction::MoveDown);
    bind_char(map, 'h', CanvasAction::MoveLeft);
    bind_char(map, 'l', CanvasAction::MoveRight);
    bind_char(map, 'w', CanvasAction::MoveWordNext);
    bind_char(map, 'b', CanvasAction::MoveWordPrev);
    bind_char(map, 'e', CanvasAction::MoveWordEnd);
    bind_sequence(map, chars(&['g', 'e']), CanvasAction::MoveWordEndPrev);
    bind_char(map, '0', CanvasAction::MoveLineStart);
    bind_char(map, '$', CanvasAction::MoveLineEnd);
    bind_sequence(map, chars(&['g', 'g']), CanvasAction::MoveFirstLine);
    bind_key_with_modifiers(map, KeyCode::Char('g'), KeyModifiers::SHIFT, CanvasAction::MoveLastLine);
    bind_key(map, KeyCode::Tab, CanvasAction::NextField);
    bind_key(map, KeyCode::BackTab, CanvasAction::PrevField);
    bind_key(map, KeyCode::Enter, CanvasAction::NextField);
    bind_char(map, 'i', CanvasAction::EnterEditMode);
    bind_char(map, 'a', CanvasAction::EnterEditModeAfter);
    bind_char(map, 'v', CanvasAction::EnterHighlightMode);
    bind_key_with_modifiers(
        map,
        KeyCode::Char('v'),
        KeyModifiers::SHIFT,
        CanvasAction::EnterHighlightModeLinewise,
    );
    bind_char(map, 'o', CanvasAction::OpenLineBelow);
    bind_key_with_modifiers(
        map,
        KeyCode::Char('o'),
        KeyModifiers::SHIFT,
        CanvasAction::OpenLineAbove,
    );
}

pub fn bind_insert_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_key(map, KeyCode::Esc, CanvasAction::ExitEditMode);
    bind_key(map, KeyCode::Backspace, CanvasAction::DeleteBackward);
    bind_key(map, KeyCode::Delete, CanvasAction::DeleteForward);
    bind_key(map, KeyCode::Left, CanvasAction::MoveLeft);
    bind_key(map, KeyCode::Right, CanvasAction::MoveRight);
    bind_key(map, KeyCode::Up, CanvasAction::MoveUp);
    bind_key(map, KeyCode::Down, CanvasAction::MoveDown);
    bind_key(map, KeyCode::Tab, CanvasAction::NextField);
    bind_key(map, KeyCode::BackTab, CanvasAction::PrevField);
    bind_suggestion_defaults(map);
}

pub fn bind_select_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_key(map, KeyCode::Esc, CanvasAction::ExitHighlightMode);
    bind_key(map, KeyCode::Up, CanvasAction::MoveUp);
    bind_key(map, KeyCode::Down, CanvasAction::MoveDown);
    bind_key(map, KeyCode::Left, CanvasAction::MoveLeft);
    bind_key(map, KeyCode::Right, CanvasAction::MoveRight);
    bind_char(map, 'k', CanvasAction::MoveUp);
    bind_char(map, 'j', CanvasAction::MoveDown);
    bind_char(map, 'h', CanvasAction::MoveLeft);
    bind_char(map, 'l', CanvasAction::MoveRight);
    bind_char(map, 'w', CanvasAction::MoveWordNext);
    bind_char(map, 'b', CanvasAction::MoveWordPrev);
    bind_char(map, 'e', CanvasAction::MoveWordEnd);
    bind_key(map, KeyCode::Tab, CanvasAction::NextField);
    bind_key(map, KeyCode::BackTab, CanvasAction::PrevField);
    bind_suggestion_defaults(map);
}

impl<O> PageSpec<O> {
    pub fn canvas_mode(mut self, mode: AppMode) -> Self {
        self.modes = modes_for_app_mode(mode);
        self.accepts_text_input = accepts_text_input(mode);
        self
    }

    pub fn canvas_editor<D>(self, editor: &FormEditor<D>) -> Self
    where
        D: DataProvider,
    {
        self.canvas_mode(editor.mode())
    }
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<CanvasAction>,
{
    pub fn canvas_defaults(self) -> Self {
        self.canvas_keymaps().canvas_text_actions()
    }

    pub fn canvas_keymaps(mut self) -> Self {
        bind_normal_defaults(self.input_registry.map_mut(modes::NORMAL.as_str()));
        bind_insert_defaults(self.input_registry.map_mut(modes::INSERT.as_str()));
        bind_select_defaults(self.input_registry.map_mut(modes::SELECT.as_str()));
        self
    }

    pub fn canvas_text_actions(mut self) -> Self {
        self.text_input_mapper = Some(text_chord_to_action::<A>);
        self
    }
}

fn bind_suggestion_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_key_with_modifiers(
        map,
        KeyCode::Char(' '),
        KeyModifiers::CONTROL,
        CanvasAction::TriggerSuggestions,
    );
    bind_key_with_modifiers(
        map,
        KeyCode::Char('n'),
        KeyModifiers::CONTROL,
        CanvasAction::SuggestionDown,
    );
    bind_key_with_modifiers(
        map,
        KeyCode::Char('p'),
        KeyModifiers::CONTROL,
        CanvasAction::SuggestionUp,
    );
    bind_key_with_modifiers(
        map,
        KeyCode::Char('y'),
        KeyModifiers::CONTROL,
        CanvasAction::SelectSuggestion,
    );
    bind_key_with_modifiers(
        map,
        KeyCode::Char('g'),
        KeyModifiers::CONTROL,
        CanvasAction::ExitSuggestions,
    );
}

fn bind_key<A>(map: &mut KeyMap<A>, code: KeyCode, action: CanvasAction)
where
    A: From<CanvasAction>,
{
    bind_key_with_modifiers(map, code, KeyModifiers::empty(), action);
}

fn bind_char<A>(map: &mut KeyMap<A>, ch: char, action: CanvasAction)
where
    A: From<CanvasAction>,
{
    bind_key(map, KeyCode::Char(ch), action);
}

fn bind_key_with_modifiers<A>(
    map: &mut KeyMap<A>,
    code: KeyCode,
    modifiers: KeyModifiers,
    action: CanvasAction,
)
where
    A: From<CanvasAction>,
{
    map.bind(vec![KeyChord::new(code, modifiers)], A::from(action));
}

fn bind_sequence<A>(map: &mut KeyMap<A>, sequence: Vec<KeyChord>, action: CanvasAction)
where
    A: From<CanvasAction>,
{
    map.bind(sequence, A::from(action));
}

fn chars(chars: &[char]) -> Vec<KeyChord> {
    chars
        .iter()
        .map(|ch| KeyChord::new(KeyCode::Char(*ch), KeyModifiers::empty()))
        .collect()
}
