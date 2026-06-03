//! Integration helpers for using the `canvas` crate with `tui-pages`.
//!
//! Enable the `canvas` feature and make your application action type implement
//! `From<CanvasAction>`. Then `.canvas_defaults()` on the builder installs the
//! standard `FormEditor` keybindings and typed-character action routing, while
//! [`PageSpec::canvas_editor`] keeps the active mode stack in sync with the
//! editor.

use crate::focus::{FocusIntent, FocusTarget};
use crate::input::{InputPipeline, InputRegistry, KeyChord, KeyMap, PipelineResponse};
use crate::runtime::{
    modes, ActionContext, ActionOutcome, KeyHook, KeyHookKind, KeyHookOutcome, ModeId, PageSpec,
    TuiEffect, TuiPagesBuilder, TuiPagesStatus,
};
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
pub use ::canvas::{
    default_vim_action_bindings, CanvasActionBinding, CanvasKeyBindings, KeyEventOutcome,
};
pub use ::canvas::keybindings::{CanvasKeyAction, KeyStroke};

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

pub trait CanvasFormEditorHost {
    fn mode(&self) -> AppMode;
    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> CanvasDispatchOutcome;
}

impl<D> CanvasFormEditorHost for FormEditor<D>
where
    D: DataProvider,
{
    fn mode(&self) -> AppMode {
        FormEditor::mode(self)
    }

    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> CanvasDispatchOutcome {
        dispatch_action(self, action)
    }
}

pub trait CanvasTextAreaHost {
    fn mode(&self) -> AppMode;
    fn input_key(&mut self, key: KeyEvent) -> TextAreaEventOutcome;
    fn exit_edit_mode(&mut self);
    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome;
}

impl<P> CanvasTextAreaHost for TextAreaState<P>
where
    P: TextAreaDataProvider,
{
    fn mode(&self) -> AppMode {
        self.editor().mode()
    }

    fn input_key(&mut self, key: KeyEvent) -> TextAreaEventOutcome {
        self.input(key)
    }

    fn exit_edit_mode(&mut self) {
        let _ = self.editor_mut().exit_edit_mode();
    }

    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome {
        execute_action_for_host_with_options(self.editor_mut(), action, false)
    }
}

pub trait CanvasTextInputHost {
    fn mode(&self) -> AppMode;
    fn text(&self) -> String;
    fn input_key(&mut self, key: KeyEvent) -> CanvasTextWidgetOutcome;
    fn set_suggestion_suffix(&mut self, suffix: String);
    fn clear_suggestion_suffix(&mut self);
    fn exit_edit_mode(&mut self);
    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome;
}

impl<P> CanvasTextInputHost for TextInputState<P>
where
    P: TextInputDataProvider,
{
    fn mode(&self) -> AppMode {
        self.editor().mode()
    }

    fn text(&self) -> String {
        TextInputState::text(self)
    }

    fn input_key(&mut self, key: KeyEvent) -> CanvasTextWidgetOutcome {
        dispatch_text_input_key(self, key)
    }

    fn set_suggestion_suffix(&mut self, suffix: String) {
        TextInputState::set_suggestion_suffix(self, suffix);
    }

    fn clear_suggestion_suffix(&mut self) {
        TextInputState::clear_suggestion_suffix(self);
    }

    fn exit_edit_mode(&mut self) {
        let _ = self.editor_mut().exit_edit_mode();
    }

    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome {
        execute_action_for_host_with_options(self.editor_mut(), action, false)
    }
}

pub trait CanvasWidgetState {
    fn canvas_form_editor(&mut self, _id: usize) -> Option<&mut dyn CanvasFormEditorHost> {
        None
    }

    fn canvas_textarea(&mut self, _focus_index: usize) -> Option<&mut dyn CanvasTextAreaHost> {
        None
    }

    fn canvas_textarea_entered(&mut self, _focus_index: usize) -> Option<&mut bool> {
        None
    }

    fn canvas_textinput(&mut self, _focus_index: usize) -> Option<&mut dyn CanvasTextInputHost> {
        None
    }

    fn canvas_textinput_entered(&mut self, _focus_index: usize) -> Option<&mut bool> {
        None
    }

    fn canvas_textinput_suggestion_suffix(
        &mut self,
        _focus_index: usize,
        _text: &str,
    ) -> Option<String> {
        None
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
    bind_canvas_crate_defaults_for_mode(map, AppMode::ReadOnly);
}

pub fn bind_insert_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_canvas_crate_defaults_for_mode(map, AppMode::Edit);
    bind_suggestion_defaults(map);
}

pub fn bind_select_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_canvas_crate_defaults_for_mode(map, AppMode::Highlight);
    bind_suggestion_defaults(map);
}

fn bind_canvas_crate_defaults_for_mode<A>(map: &mut KeyMap<A>, mode: AppMode)
where
    A: From<CanvasAction>,
{
    for binding in default_vim_action_bindings()
        .into_iter()
        .filter(|binding| binding.mode == mode)
    {
        let sequence = binding
            .sequence
            .into_iter()
            .map(|stroke| KeyChord::new(stroke.code, stroke.modifiers))
            .collect::<Vec<_>>();
        map.bind(sequence, A::from(binding.action));
    }
}

fn canvas_action_pipeline(timeout_ms: u64) -> InputPipeline<CanvasAction> {
    let mut registry = InputRegistry::empty();
    bind_normal_defaults(registry.map_mut(modes::NORMAL.as_str()));
    bind_insert_defaults(registry.map_mut(modes::INSERT.as_str()));
    bind_select_defaults(registry.map_mut(modes::SELECT.as_str()));
    InputPipeline::new(registry, timeout_ms)
}

fn normalize_shift(mut key: KeyEvent) -> KeyEvent {
    if matches!(key.code, KeyCode::Char(_)) && key.modifiers == KeyModifiers::SHIFT {
        key.modifiers = KeyModifiers::NONE;
    }
    key
}

fn focused_canvas_field<V, O>(ctx: &ActionContext<V, O>, index: usize) -> bool {
    matches!(
        ctx.focus.as_ref(),
        Some(FocusTarget::CanvasField(field) | FocusTarget::InternalCanvasField(field))
            if *field == index
    )
}

fn focus_intent_for_top_level_key<O, M>(key: KeyEvent) -> Option<FocusIntent<O, M>> {
    match (key.code, key.modifiers) {
        (KeyCode::Down | KeyCode::Tab, _) => Some(FocusIntent::ExitCanvasForward),
        (KeyCode::Char('j') | KeyCode::Char('l'), modifiers)
            if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
        {
            Some(FocusIntent::ExitCanvasForward)
        }
        (KeyCode::Up | KeyCode::BackTab, _) => Some(FocusIntent::ExitCanvasBackward),
        (KeyCode::Char('k') | KeyCode::Char('h'), modifiers)
            if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT =>
        {
            Some(FocusIntent::ExitCanvasBackward)
        }
        _ => None,
    }
}

fn hook_outcome<V, A, O, M>(
    status: TuiPagesStatus<A>,
    outcome: ActionOutcome<V, O, M>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    Some(KeyHookOutcome { status, outcome })
}

fn hook_focus_outcome<V, A, O, M>(
    intent: FocusIntent<O, M>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    hook_outcome(
        TuiPagesStatus::ActionHandled,
        ActionOutcome::effect(TuiEffect::Focus(intent)),
    )
}

fn hook_status_outcome<V, A, O, M>(
    status: TuiPagesStatus<A>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    hook_outcome(status, ActionOutcome::none())
}

fn form_host_dispatch_hook_outcome<V, A, O, M>(
    outcome: CanvasDispatchOutcome,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    match outcome {
        CanvasDispatchOutcome::Applied(_) => {
            hook_status_outcome(TuiPagesStatus::ActionHandled)
        }
        CanvasDispatchOutcome::Focus(FocusIntent::ExitCanvasForward) => {
            hook_focus_outcome(FocusIntent::ExitCanvasForward)
        }
        CanvasDispatchOutcome::Focus(FocusIntent::ExitCanvasBackward) => {
            hook_focus_outcome(FocusIntent::ExitCanvasBackward)
        }
        CanvasDispatchOutcome::Focus(_) => hook_status_outcome(TuiPagesStatus::ActionHandled),
    }
}

fn widget_action_hook_outcome<V, A, O, M>(
    outcome: HostActionOutcome,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    match outcome {
        HostActionOutcome::Applied(_) => hook_status_outcome(TuiPagesStatus::ActionHandled),
        HostActionOutcome::ExitCanvas(boundary) => {
            hook_focus_outcome(focus_intent_for_boundary(boundary))
        }
    }
}

fn pipeline_hook_outcome<V, A, O, M>(
    response: PipelineResponse<CanvasAction>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    match response {
        PipelineResponse::Wait(_) => hook_status_outcome(TuiPagesStatus::Waiting(Vec::new())),
        PipelineResponse::Cancel => hook_status_outcome(TuiPagesStatus::Cancelled),
        PipelineResponse::Execute(_) | PipelineResponse::Type(_) => None,
    }
}

fn refresh_textinput_suggestion_suffix<S>(state: &mut S, focus_index: usize)
where
    S: CanvasWidgetState,
{
    let Some(text) = state.canvas_textinput(focus_index).map(|input| input.text()) else {
        return;
    };
    let suffix = state.canvas_textinput_suggestion_suffix(focus_index, &text);
    let Some(input) = state.canvas_textinput(focus_index) else {
        return;
    };
    if let Some(suffix) = suffix {
        input.set_suggestion_suffix(suffix);
    } else {
        input.clear_suggestion_suffix();
    }
}

pub(crate) fn dispatch_canvas_key_hook<V, A, S, O, M>(
    kind: &mut KeyHookKind,
    key: KeyEvent,
    ctx: ActionContext<V, O>,
    state: &mut S,
) -> Option<KeyHookOutcome<V, A, O, M>>
where
    S: CanvasWidgetState,
{
    match kind {
        KeyHookKind::CanvasFormEditor { id, pipeline } => {
            if !ctx.focus.as_ref().is_some_and(FocusTarget::is_canvas) {
                return None;
            }

            let editor = state.canvas_form_editor(*id)?;
            let mode = editor.mode();
            let modes = modes_for_app_mode(mode);
            match pipeline.process(key, &modes, accepts_text_input(mode)) {
                PipelineResponse::Execute(action) => {
                    form_host_dispatch_hook_outcome(editor.dispatch_canvas_action(action))
                }
                PipelineResponse::Type(chord) if accepts_text_input(mode) => {
                    text_chord_to_canvas_action(chord).and_then(|action| {
                        form_host_dispatch_hook_outcome(editor.dispatch_canvas_action(action))
                    })
                }
                response => pipeline_hook_outcome(response),
            }
        }
        KeyHookKind::CanvasTextArea {
            focus_index,
            pipeline,
        } => {
            if !focused_canvas_field(&ctx, *focus_index) {
                if let Some(entered) = state.canvas_textarea_entered(*focus_index) {
                    *entered = false;
                }
                return None;
            }

            let is_entered = state
                .canvas_textarea_entered(*focus_index)
                .is_some_and(|entered| *entered);
            if !is_entered {
                if key.kind != KeyEventKind::Press {
                    return None;
                }
                if matches!(key.code, KeyCode::Enter) {
                    if let Some(entered) = state.canvas_textarea_entered(*focus_index) {
                        *entered = true;
                    }
                    return hook_status_outcome(TuiPagesStatus::ActionHandled);
                }
                return focus_intent_for_top_level_key(key).and_then(hook_focus_outcome);
            }

            let mode = state.canvas_textarea(*focus_index)?.mode();
            if mode == AppMode::Edit {
                return match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => None,
                    (KeyCode::Esc, _) => {
                        state.canvas_textarea(*focus_index)?.exit_edit_mode();
                        hook_status_outcome(TuiPagesStatus::ActionHandled)
                    }
                    _ => match state
                        .canvas_textarea(*focus_index)?
                        .input_key(normalize_shift(key))
                    {
                        TextAreaEventOutcome::Handled => {
                            hook_status_outcome(TuiPagesStatus::TextHandled)
                        }
                        TextAreaEventOutcome::Ignored => None,
                    },
                };
            }

            if matches!(key.code, KeyCode::Esc) && key.kind == KeyEventKind::Press {
                if let Some(entered) = state.canvas_textarea_entered(*focus_index) {
                    *entered = false;
                }
                return hook_status_outcome(TuiPagesStatus::ActionHandled);
            }

            let modes = modes_for_app_mode(mode);
            match pipeline.process(key, &modes, accepts_text_input(mode)) {
                PipelineResponse::Execute(action) => widget_action_hook_outcome(
                    state.canvas_textarea(*focus_index)?.dispatch_canvas_action(action),
                ),
                response => pipeline_hook_outcome(response),
            }
        }
        KeyHookKind::CanvasTextInput {
            focus_index,
            pipeline,
        } => {
            if !focused_canvas_field(&ctx, *focus_index) {
                if let Some(entered) = state.canvas_textinput_entered(*focus_index) {
                    *entered = false;
                }
                return None;
            }

            let is_entered = state
                .canvas_textinput_entered(*focus_index)
                .is_some_and(|entered| *entered);
            if !is_entered {
                if key.kind != KeyEventKind::Press {
                    return None;
                }
                if matches!(key.code, KeyCode::Enter) {
                    if let Some(entered) = state.canvas_textinput_entered(*focus_index) {
                        *entered = true;
                    }
                    return hook_status_outcome(TuiPagesStatus::ActionHandled);
                }
                return focus_intent_for_top_level_key(key).and_then(hook_focus_outcome);
            }

            let mode = state.canvas_textinput(*focus_index)?.mode();
            if mode == AppMode::Edit {
                return match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => None,
                    (KeyCode::Esc, _) => {
                        state.canvas_textinput(*focus_index)?.exit_edit_mode();
                        hook_status_outcome(TuiPagesStatus::ActionHandled)
                    }
                    _ => match state
                        .canvas_textinput(*focus_index)?
                        .input_key(normalize_shift(key))
                    {
                        CanvasTextWidgetOutcome::Handled => {
                            refresh_textinput_suggestion_suffix(state, *focus_index);
                            hook_status_outcome(TuiPagesStatus::TextHandled)
                        }
                        CanvasTextWidgetOutcome::Submitted => {
                            if let Some(entered) = state.canvas_textinput_entered(*focus_index) {
                                *entered = false;
                            }
                            hook_focus_outcome(FocusIntent::ExitCanvasForward)
                        }
                        CanvasTextWidgetOutcome::Focus(intent) => {
                            if let Some(entered) = state.canvas_textinput_entered(*focus_index) {
                                *entered = false;
                            }
                            match intent {
                                FocusIntent::ExitCanvasForward => {
                                    hook_focus_outcome(FocusIntent::ExitCanvasForward)
                                }
                                FocusIntent::ExitCanvasBackward => {
                                    hook_focus_outcome(FocusIntent::ExitCanvasBackward)
                                }
                                _ => hook_status_outcome(TuiPagesStatus::ActionHandled),
                            }
                        }
                        CanvasTextWidgetOutcome::NotHandled => None,
                    },
                };
            }

            if matches!(key.code, KeyCode::Esc) && key.kind == KeyEventKind::Press {
                if let Some(entered) = state.canvas_textinput_entered(*focus_index) {
                    *entered = false;
                }
                return hook_status_outcome(TuiPagesStatus::ActionHandled);
            }

            let modes = modes_for_app_mode(mode);
            match pipeline.process(key, &modes, accepts_text_input(mode)) {
                PipelineResponse::Execute(action) => widget_action_hook_outcome(
                    state.canvas_textinput(*focus_index)?.dispatch_canvas_action(action),
                ),
                response => pipeline_hook_outcome(response),
            }
        }
    }
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
        self.canvas_keybindings().canvas_text_actions()
    }

    pub fn canvas_keybindings(mut self) -> Self {
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

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    S: CanvasWidgetState,
{
    pub fn canvas_form_editor(mut self, id: usize) -> Self {
        self.key_hooks.push(KeyHook {
            kind: KeyHookKind::CanvasFormEditor {
                id,
                pipeline: canvas_action_pipeline(self.input_timeout_ms),
            },
            dispatch: dispatch_canvas_key_hook::<V, A, S, O, M>,
        });
        self
    }

    pub fn canvas_textarea_widget(mut self, focus_index: usize) -> Self {
        self.key_hooks.push(KeyHook {
            kind: KeyHookKind::CanvasTextArea {
                focus_index,
                pipeline: canvas_action_pipeline(self.input_timeout_ms),
            },
            dispatch: dispatch_canvas_key_hook::<V, A, S, O, M>,
        });
        self
    }

    pub fn canvas_textinput_widget(mut self, focus_index: usize) -> Self {
        self.key_hooks.push(KeyHook {
            kind: KeyHookKind::CanvasTextInput {
                focus_index,
                pipeline: canvas_action_pipeline(self.input_timeout_ms),
            },
            dispatch: dispatch_canvas_key_hook::<V, A, S, O, M>,
        });
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
