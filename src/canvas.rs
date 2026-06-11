//! Integration helpers for using the `canvas` crate with `tui-pages`.
//!
//! Enable the `canvas` feature and make your application action type implement
//! `From<CanvasAction>`. Then `.canvas_defaults()` on the builder installs the
//! standard `FormEditor` keybindings and typed-character action routing, while
//! [`PageSpec::canvas_editor`] keeps the active mode stack in sync with the
//! editor.

use crate::focus::{FocusIntent, FocusTarget};
use crate::input::{
    BindableActionInfo, BindingCatalog, BindingConflict, BindingInfo, BindingLayer, BindingSource,
    CanvasRoutingPrecedence, InputPipeline, InputRegistry, KeyChord, KeyMap, PipelineResponse,
};
use crate::runtime::{
    modes, ActionContext, ActionOutcome, InputLayerContext, KeyHook, KeyHookKind, KeyHookOutcome,
    KeyHookRouting, ModeId, PageSpec, TuiEffect, TuiPagesBuilder, TuiPagesStatus,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, Frame};

// The `canvas` feature enables full canvas support — every surface below is
// available unconditionally. We do not split canvas into sub-features.

// --- Base surface ---
pub use ::canvas::{
    ActionResult, AppMode, CanvasAction, DataProvider, EditorState, TextFormEventOutcome,
    TextFormState,
};
pub use ::canvas::integration::focus_handoff::{
    execute_action_for_host, execute_action_for_host_with_options, BoundaryExit, HostActionOutcome,
};

// --- Keymap-driven host handoff ---
pub use ::canvas::integration::focus_handoff::{
    boundary_from_key_outcome, handle_key_event_for_host, key_outcome_for_vertical_navigation,
    map_key_event_outcome_for_host, HostKeyEventOutcome,
};
pub use ::canvas::{
    default_builtin_action_bindings, default_emacs_action_bindings,
    default_helix_action_bindings, default_vim_action_bindings, display_binding, preset,
    BuiltinCanvasKeybindingPreset, CanvasActionBinding, CanvasActionKeyBinding,
    CanvasKeyBindingEntry, CanvasKeyBindings, CanvasKeybindingConflictKind,
    CanvasKeybindingProfile, KeyEventOutcome,
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
pub use ::canvas::validation::limits::{CountMode, LimitCheckResult};

// --- Computed fields ---
pub use ::canvas::{ComputedContext, ComputedProvider, ComputedState};

// --- GUI: renderers, themes, display options ---
pub use ::canvas::{
    render_canvas, render_canvas_default, render_canvas_with_options, CanvasDisplayOptions,
    CanvasTheme, DefaultCanvasTheme, OverflowMode,
};

// Crossterm terminal-input session helpers (raw mode, bracketed paste, mouse
// capture) — the canvas-side complement to [`crate::terminal`] for apps wiring
// up text widgets that need paste support.
pub use ::canvas::integration::crossterm_input::{
    CrosstermInputGuard, CrosstermInputOptions, CrosstermInputSession,
};

// --- Text area ---
pub use ::canvas::{
    TextArea, TextAreaDataProvider, TextAreaProvider, TextAreaState,
};
pub use ::canvas::textarea::{
    TextAreaCommandLineState, TextAreaEventOutcome, TextAreaLineNumberMode, TextAreaSearchMatch,
    TextOverflowMode,
};

// --- Command line ---
pub use ::canvas::{
    parse_command_args, parse_command_line, CommandLine, CommandLineCommand,
    CommandLineCommandInvocation, CommandLineDispatchError, CommandLineEventOutcome,
    CommandLineMode, CommandLineParseError, CommandLineParsedCommand, CommandLinePlacement,
    CommandLineRegistry, CommandLineRegistrationError, CommandLineState, CommandLineSubmit,
};

// --- Text input ---
pub use ::canvas::{
    TextInput, TextInputDataProvider, TextInputEventOutcome, TextInputProvider,
    TextInputState,
};

pub type FormEditor<D> = TextFormState<D>;
pub type FormInputEventOutcome = TextFormEventOutcome;
pub type TextAreaEditor<P> = TextAreaState<P>;
pub type TextInputEditor<P> = TextInputState<P>;

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
    fn has_keybindings(&self) -> bool;
    fn use_keybinding_preset(&mut self, preset: BuiltinCanvasKeybindingPreset);
    /// Whether a multi-key command (sequence/count/operator) is mid-flight, so
    /// the runtime keeps routing keys here instead of letting the global keymap
    /// claim the next stroke. Defaults to `false` for hosts without modal state.
    fn is_sequence_pending(&self) -> bool {
        false
    }
    fn input_key(&mut self, key: KeyEvent) -> CanvasKeyDispatchOutcome;
    /// Insert pasted (bracketed-paste) text. Returns whether anything was
    /// inserted.
    fn paste(&mut self, text: &str) -> bool;
    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> CanvasDispatchOutcome;
}

impl<D> CanvasFormEditorHost for FormEditor<D>
where
    D: DataProvider,
{
    fn mode(&self) -> AppMode {
        self.core().mode()
    }

    fn has_keybindings(&self) -> bool {
        self.core().has_keybindings()
    }

    fn use_keybinding_preset(&mut self, preset: BuiltinCanvasKeybindingPreset) {
        FormEditor::use_keybinding_preset(self, preset);
    }

    fn is_sequence_pending(&self) -> bool {
        FormEditor::is_sequence_pending(self)
    }

    fn input_key(&mut self, key: KeyEvent) -> CanvasKeyDispatchOutcome {
        dispatch_key_event(self, key)
    }

    fn paste(&mut self, text: &str) -> bool {
        matches!(FormEditor::paste(self, text), TextFormEventOutcome::Handled)
    }

    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> CanvasDispatchOutcome {
        dispatch_action(self, action)
    }
}

pub trait CanvasTextAreaHost {
    fn mode(&self) -> AppMode;
    fn has_keybindings(&self) -> bool;
    fn use_keybinding_preset(&mut self, preset: BuiltinCanvasKeybindingPreset);
    /// Whether a multi-key command (sequence/count/operator/literal-char capture)
    /// is mid-flight, so the runtime keeps routing keys here instead of letting
    /// the global keymap claim the next stroke.
    fn is_sequence_pending(&self) -> bool {
        false
    }
    fn commandline_enabled(&self) -> bool {
        false
    }
    fn boundary_for_key(&self, key: &KeyEvent) -> Option<BoundaryExit>;
    fn input_key(&mut self, key: KeyEvent) -> TextAreaEventOutcome;
    /// Insert pasted (bracketed-paste) text. Returns whether anything was
    /// inserted.
    fn paste(&mut self, text: &str) -> bool;
    fn exit_edit_mode(&mut self);
    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome;
}

impl<P> CanvasTextAreaHost for TextAreaState<P>
where
    P: TextAreaDataProvider,
{
    fn mode(&self) -> AppMode {
        self.mode()
    }

    fn has_keybindings(&self) -> bool {
        self.core().has_keybindings()
    }

    fn use_keybinding_preset(&mut self, preset: BuiltinCanvasKeybindingPreset) {
        TextAreaState::use_keybinding_preset(self, preset);
    }

    fn is_sequence_pending(&self) -> bool {
        TextAreaState::is_sequence_pending(self)
    }

    fn commandline_enabled(&self) -> bool {
        self.commandline().is_some()
    }

    fn boundary_for_key(&self, key: &KeyEvent) -> Option<BoundaryExit> {
        text_area_boundary_for_key(self, key)
    }

    fn input_key(&mut self, key: KeyEvent) -> TextAreaEventOutcome {
        if self.core().has_keybindings() || self.commandline_enabled() {
            match self.handle_key_event(key) {
                KeyEventOutcome::Consumed(_)
                | KeyEventOutcome::Pending
                | KeyEventOutcome::ExitTop
                | KeyEventOutcome::ExitBottom => TextAreaEventOutcome::Handled,
                KeyEventOutcome::NotMatched => TextAreaEventOutcome::Ignored,
            }
        } else {
            self.input(key)
        }
    }

    fn paste(&mut self, text: &str) -> bool {
        matches!(TextAreaState::paste(self, text), TextAreaEventOutcome::Handled)
    }

    fn exit_edit_mode(&mut self) {
        let _ = self.core_mut().exit_edit_mode();
    }

    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome {
        HostActionOutcome::Applied(self.core_mut().execute(action))
    }
}

pub trait CanvasTextInputHost {
    fn mode(&self) -> AppMode;
    fn text(&self) -> String;
    fn input_key(&mut self, key: KeyEvent) -> CanvasTextWidgetOutcome;
    /// Insert pasted (bracketed-paste) text. Returns whether anything was
    /// inserted.
    fn paste(&mut self, text: &str) -> bool;
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
        self.mode()
    }

    fn text(&self) -> String {
        TextInputState::text(self)
    }

    fn input_key(&mut self, key: KeyEvent) -> CanvasTextWidgetOutcome {
        dispatch_text_input_key(self, key)
    }

    fn paste(&mut self, text: &str) -> bool {
        matches!(
            TextInputState::paste(self, text),
            TextInputEventOutcome::Handled
        )
    }

    fn set_suggestion_suffix(&mut self, suffix: String) {
        TextInputState::set_suggestion_suffix(self, suffix);
    }

    fn clear_suggestion_suffix(&mut self) {
        TextInputState::clear_suggestion_suffix(self);
    }

    fn exit_edit_mode(&mut self) {
        let _ = self.form_mut().exit_edit_mode();
    }

    fn dispatch_canvas_action(&mut self, action: CanvasAction) -> HostActionOutcome {
        execute_action_for_host_with_options(self.form_mut(), action, false)
    }
}

pub trait CanvasWidgetState {
    fn canvas_form_editor_ref(&self, _id: usize) -> Option<&dyn CanvasFormEditorHost> {
        None
    }

    fn canvas_form_editor(&mut self, _id: usize) -> Option<&mut dyn CanvasFormEditorHost> {
        None
    }

    fn canvas_textarea_ref(&self, _focus_index: usize) -> Option<&dyn CanvasTextAreaHost> {
        None
    }

    fn canvas_textarea(&mut self, _focus_index: usize) -> Option<&mut dyn CanvasTextAreaHost> {
        None
    }

    fn canvas_textarea_entered_ref(&self, _focus_index: usize) -> Option<&bool> {
        None
    }

    fn canvas_textarea_entered(&mut self, _focus_index: usize) -> Option<&mut bool> {
        None
    }

    fn canvas_textinput_ref(&self, _focus_index: usize) -> Option<&dyn CanvasTextInputHost> {
        None
    }

    fn canvas_textinput(&mut self, _focus_index: usize) -> Option<&mut dyn CanvasTextInputHost> {
        None
    }

    fn canvas_textinput_entered_ref(&self, _focus_index: usize) -> Option<&bool> {
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
        AppMode::Ins => modes::INSERT,
        AppMode::Sel => modes::SELECT,
        AppMode::Command => modes::COMMAND,
        AppMode::General => modes::GENERAL,
        AppMode::Nor => modes::NORMAL,
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
    matches!(mode, AppMode::Ins | AppMode::Command)
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
    let opts = CanvasDisplayOptions::default();
    render_canvas_with_suggestions_with_options(frame, frame_area, canvas_area, editor, theme, opts)
}

pub fn render_canvas_with_suggestions_with_options<T, D>(
    frame: &mut Frame,
    frame_area: Rect,
    canvas_area: Rect,
    editor: &FormEditor<D>,
    theme: &T,
    opts: CanvasDisplayOptions,
) -> Option<Rect>
where
    T: CanvasTheme,
    D: DataProvider,
{
    let input_rect = render_canvas_with_options(frame, canvas_area, editor, theme, opts);
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

pub fn render_canvas_with_suggestions_default_options<D>(
    frame: &mut Frame,
    frame_area: Rect,
    canvas_area: Rect,
    editor: &FormEditor<D>,
    opts: CanvasDisplayOptions,
) -> Option<Rect>
where
    D: DataProvider,
{
    let theme = DefaultCanvasTheme;
    render_canvas_with_suggestions_with_options(
        frame,
        frame_area,
        canvas_area,
        editor,
        &theme,
        opts,
    )
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

    if textarea.commandline().is_some() {
        match textarea.handle_key_event(event) {
            KeyEventOutcome::Consumed(_) | KeyEventOutcome::Pending => {
                CanvasTextWidgetOutcome::Handled
            }
            KeyEventOutcome::ExitTop => {
                CanvasTextWidgetOutcome::Focus(focus_intent_for_boundary(BoundaryExit::Top))
            }
            KeyEventOutcome::ExitBottom => {
                CanvasTextWidgetOutcome::Focus(focus_intent_for_boundary(BoundaryExit::Bottom))
            }
            KeyEventOutcome::NotMatched => CanvasTextWidgetOutcome::NotHandled,
        }
    } else {
        match textarea.input(event) {
            TextAreaEventOutcome::Handled => CanvasTextWidgetOutcome::Handled,
            TextAreaEventOutcome::Ignored => CanvasTextWidgetOutcome::NotHandled,
        }
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
    bind_builtin_keymaps(
        BuiltinCanvasKeybindingPreset::Vim,
        normal,
        insert,
        select,
    );
}

pub fn bind_builtin_keymaps<A>(
    preset: BuiltinCanvasKeybindingPreset,
    normal: &mut KeyMap<A>,
    insert: &mut KeyMap<A>,
    select: &mut KeyMap<A>,
)
where
    A: From<CanvasAction>,
{
    bind_builtin_defaults_for_mode(preset, normal, AppMode::Nor);
    bind_builtin_defaults_for_mode(preset, insert, AppMode::Ins);
    bind_suggestion_defaults(insert);
    bind_builtin_defaults_for_mode(preset, select, AppMode::Sel);
    bind_suggestion_defaults(select);
}

pub fn bind_normal_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_builtin_defaults_for_mode(BuiltinCanvasKeybindingPreset::Vim, map, AppMode::Nor);
}

pub fn bind_insert_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_builtin_defaults_for_mode(BuiltinCanvasKeybindingPreset::Vim, map, AppMode::Ins);
    bind_suggestion_defaults(map);
}

pub fn bind_select_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<CanvasAction>,
{
    bind_builtin_defaults_for_mode(BuiltinCanvasKeybindingPreset::Vim, map, AppMode::Sel);
    bind_suggestion_defaults(map);
}

fn bind_builtin_defaults_for_mode<A>(
    preset: BuiltinCanvasKeybindingPreset,
    map: &mut KeyMap<A>,
    mode: AppMode,
)
where
    A: From<CanvasAction>,
{
    for binding in default_builtin_action_bindings(preset)
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

fn canvas_action_pipeline_with_preset(
    preset: BuiltinCanvasKeybindingPreset,
    timeout_ms: u64,
) -> InputPipeline<CanvasAction> {
    let mut registry = InputRegistry::empty();
    bind_builtin_defaults_for_mode(preset, registry.map_mut(modes::NORMAL.as_str()), AppMode::Nor);
    bind_builtin_defaults_for_mode(preset, registry.map_mut(modes::INSERT.as_str()), AppMode::Ins);
    bind_suggestion_defaults(registry.map_mut(modes::INSERT.as_str()));
    bind_builtin_defaults_for_mode(preset, registry.map_mut(modes::SELECT.as_str()), AppMode::Sel);
    bind_suggestion_defaults(registry.map_mut(modes::SELECT.as_str()));
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

fn input_layer_context_for_mode(mode: AppMode) -> InputLayerContext {
    if accepts_text_input(mode) {
        InputLayerContext::Text
    } else {
        InputLayerContext::Command
    }
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
    routing: KeyHookRouting,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    Some(KeyHookOutcome {
        status,
        outcome,
        routing,
    })
}

fn hook_focus_outcome<V, A, O, M>(
    intent: FocusIntent<O, M>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    hook_outcome(
        TuiPagesStatus::ActionHandled,
        ActionOutcome::effect(TuiEffect::Focus(intent)),
        KeyHookRouting::Handled,
    )
}

fn hook_status_outcome<V, A, O, M>(
    status: TuiPagesStatus<A>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    hook_outcome(status, ActionOutcome::none(), KeyHookRouting::Handled)
}

fn hook_pending_outcome<V, A, O, M>(
    status: TuiPagesStatus<A>,
) -> Option<KeyHookOutcome<V, A, O, M>> {
    hook_outcome(status, ActionOutcome::none(), KeyHookRouting::Pending)
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
        PipelineResponse::Wait(_) => hook_pending_outcome(TuiPagesStatus::Waiting(Vec::new())),
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

pub(crate) fn canvas_hook_context<V, S, O>(
    kind: &KeyHookKind,
    ctx: &ActionContext<V, O>,
    state: &S,
) -> Option<InputLayerContext>
where
    S: CanvasWidgetState,
{
    match kind {
        KeyHookKind::CanvasFormEditor { id, .. } => {
            if !ctx.focus.as_ref().is_some_and(FocusTarget::is_canvas) {
                return None;
            }
            state
                .canvas_form_editor_ref(*id)
                .map(|editor| input_layer_context_for_mode(editor.mode()))
        }
        KeyHookKind::CanvasTextArea { focus_index, .. } => {
            if !focused_canvas_field(ctx, *focus_index) {
                return None;
            }
            if !state
                .canvas_textarea_entered_ref(*focus_index)
                .is_some_and(|entered| *entered)
            {
                return Some(InputLayerContext::Command);
            }
            state
                .canvas_textarea_ref(*focus_index)
                .map(|textarea| input_layer_context_for_mode(textarea.mode()))
        }
        KeyHookKind::CanvasTextInput { focus_index, .. } => {
            if !focused_canvas_field(ctx, *focus_index) {
                return None;
            }
            if !state
                .canvas_textinput_entered_ref(*focus_index)
                .is_some_and(|entered| *entered)
            {
                return Some(InputLayerContext::Command);
            }
            state
                .canvas_textinput_ref(*focus_index)
                .map(|input| input_layer_context_for_mode(input.mode()))
        }
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
        KeyHookKind::CanvasFormEditor {
            id,
            preset,
        } => {
            if !ctx.focus.as_ref().is_some_and(FocusTarget::is_canvas) {
                return None;
            }

            let editor = state.canvas_form_editor(*id)?;
            if !editor.has_keybindings() {
                editor.use_keybinding_preset(*preset);
            }

            let outcome = editor.input_key(normalize_shift(key));
            // A consumed key that left the editor mid-sequence (operator/count
            // awaiting more input) is reported as Waiting so the orchestrator
            // keeps this hook as the sticky owner of the keys that follow.
            let pending = state.canvas_form_editor(*id)?.is_sequence_pending();
            match outcome {
                CanvasKeyDispatchOutcome::Consumed(_) if pending => {
                    hook_pending_outcome(TuiPagesStatus::Waiting(Vec::new()))
                }
                CanvasKeyDispatchOutcome::Consumed(_) => {
                    hook_status_outcome(TuiPagesStatus::ActionHandled)
                }
                CanvasKeyDispatchOutcome::PendingSequence => {
                    hook_pending_outcome(TuiPagesStatus::Waiting(Vec::new()))
                }
                CanvasKeyDispatchOutcome::NotHandled => None,
                CanvasKeyDispatchOutcome::Focus(FocusIntent::ExitCanvasForward) => {
                    hook_focus_outcome(FocusIntent::ExitCanvasForward)
                }
                CanvasKeyDispatchOutcome::Focus(FocusIntent::ExitCanvasBackward) => {
                    hook_focus_outcome(FocusIntent::ExitCanvasBackward)
                }
                CanvasKeyDispatchOutcome::Focus(_) => {
                    hook_status_outcome(TuiPagesStatus::ActionHandled)
                }
            }
        }
        KeyHookKind::CanvasTextArea {
            focus_index,
            preset,
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
                    if let Some(textarea) = state.canvas_textarea(*focus_index) {
                        if !textarea.has_keybindings() {
                            textarea.use_keybinding_preset(*preset);
                        }
                        textarea.exit_edit_mode();
                    }
                    return hook_status_outcome(TuiPagesStatus::ActionHandled);
                }
                return focus_intent_for_top_level_key(key).and_then(hook_focus_outcome);
            }

            if let Some(textarea) = state.canvas_textarea(*focus_index) {
                if !textarea.has_keybindings() {
                    textarea.use_keybinding_preset(*preset);
                }
            }

            let mode = state.canvas_textarea(*focus_index)?.mode();
            if mode == AppMode::Ins {
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

            if matches!((key.code, key.modifiers), (KeyCode::Char('g'), KeyModifiers::CONTROL))
                && key.kind == KeyEventKind::Press
            {
                if let Some(entered) = state.canvas_textarea_entered(*focus_index) {
                    *entered = false;
                }
                return hook_focus_outcome(FocusIntent::ExitCanvasForward);
            }

            if let Some(boundary) = state.canvas_textarea(*focus_index)?.boundary_for_key(&key) {
                return hook_focus_outcome(focus_intent_for_boundary(boundary));
            }

            let outcome = state
                .canvas_textarea(*focus_index)?
                .input_key(normalize_shift(key));
            // A consumed key that left the editor mid-sequence (operator/count or
            // an `f`/`r` literal-char capture) is reported as Waiting so the
            // orchestrator keeps this hook as the sticky owner of the next keys.
            let pending = state.canvas_textarea(*focus_index)?.is_sequence_pending();
            match outcome {
                TextAreaEventOutcome::Handled if pending => {
                    return hook_pending_outcome(TuiPagesStatus::Waiting(Vec::new()));
                }
                TextAreaEventOutcome::Handled => {
                    return hook_status_outcome(TuiPagesStatus::TextHandled);
                }
                TextAreaEventOutcome::Ignored => {}
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
            if mode == AppMode::Ins {
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

/// Forward a bracketed-paste payload to whichever canvas widget this hook owns,
/// but only when that widget currently holds focus (and, for the text widgets,
/// has been entered for editing). Mirrors the focus gating in
/// [`dispatch_canvas_key_hook`] so paste routes exactly like typed input.
pub(crate) fn dispatch_canvas_paste_hook<V, A, S, O, M>(
    kind: &mut KeyHookKind,
    text: &str,
    ctx: ActionContext<V, O>,
    state: &mut S,
) -> Option<KeyHookOutcome<V, A, O, M>>
where
    S: CanvasWidgetState,
{
    let handled = match kind {
        KeyHookKind::CanvasFormEditor { id, .. } => {
            if !ctx.focus.as_ref().is_some_and(FocusTarget::is_canvas) {
                return None;
            }
            state.canvas_form_editor(*id)?.paste(text)
        }
        KeyHookKind::CanvasTextArea { focus_index, .. } => {
            if !focused_canvas_field(&ctx, *focus_index)
                || !state
                    .canvas_textarea_entered(*focus_index)
                    .is_some_and(|entered| *entered)
            {
                return None;
            }
            state.canvas_textarea(*focus_index)?.paste(text)
        }
        KeyHookKind::CanvasTextInput { focus_index, .. } => {
            if !focused_canvas_field(&ctx, *focus_index)
                || !state
                    .canvas_textinput_entered(*focus_index)
                    .is_some_and(|entered| *entered)
            {
                return None;
            }
            state.canvas_textinput(*focus_index)?.paste(text)
        }
    };

    if handled {
        hook_status_outcome(TuiPagesStatus::TextHandled)
    } else {
        None
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
    pub fn canvas_form_editor(self, id: usize) -> Self {
        self.canvas_form_editor_with_preset(id, BuiltinCanvasKeybindingPreset::Vim)
    }

    pub fn canvas_form_editor_with_preset(
        mut self,
        id: usize,
        preset: BuiltinCanvasKeybindingPreset,
    ) -> Self {
        self.key_hooks.push(KeyHook {
            kind: KeyHookKind::CanvasFormEditor {
                id,
                preset,
            },
            context: canvas_hook_context::<V, S, O>,
            dispatch: dispatch_canvas_key_hook::<V, A, S, O, M>,
            paste: dispatch_canvas_paste_hook::<V, A, S, O, M>,
        });
        self
    }

    pub fn canvas_textarea_widget(self, focus_index: usize) -> Self {
        self.canvas_textarea_widget_with_preset(focus_index, BuiltinCanvasKeybindingPreset::Vim)
    }

    pub fn canvas_textarea_widget_with_preset(
        mut self,
        focus_index: usize,
        preset: BuiltinCanvasKeybindingPreset,
    ) -> Self {
        self.key_hooks.push(KeyHook {
            kind: KeyHookKind::CanvasTextArea {
                focus_index,
                preset,
                pipeline: canvas_action_pipeline_with_preset(preset, self.input_timeout_ms),
            },
            context: canvas_hook_context::<V, S, O>,
            dispatch: dispatch_canvas_key_hook::<V, A, S, O, M>,
            paste: dispatch_canvas_paste_hook::<V, A, S, O, M>,
        });
        self
    }

    pub fn canvas_textinput_widget(self, focus_index: usize) -> Self {
        self.canvas_textinput_widget_with_preset(focus_index, BuiltinCanvasKeybindingPreset::Vim)
    }

    pub fn canvas_textinput_widget_with_preset(
        mut self,
        focus_index: usize,
        preset: BuiltinCanvasKeybindingPreset,
    ) -> Self {
        self.key_hooks.push(KeyHook {
            kind: KeyHookKind::CanvasTextInput {
                focus_index,
                pipeline: canvas_action_pipeline_with_preset(preset, self.input_timeout_ms),
            },
            context: canvas_hook_context::<V, S, O>,
            dispatch: dispatch_canvas_key_hook::<V, A, S, O, M>,
            paste: dispatch_canvas_paste_hook::<V, A, S, O, M>,
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

// --- Binding introspection (catalogs, bindable-action metadata, overlap) ---
//
// These helpers expose the canvas keybinding *defaults* as a source-tagged
// [`BindingCatalog`], independent of whether they were mirrored into the global
// keymap. They feed help screens, `:bindings` panels, and conflict diagnostics;
// none of them are on the input hot path.

/// Modes a non-suggestion canvas action can be bound in.
const CANVAS_EDIT_MODES: &[&str] = &["nor", "ins", "sel"];
/// Modes the suggestion-dropdown actions are bound in.
const CANVAS_SUGGESTION_MODES: &[&str] = &["ins", "sel"];

/// The config-facing name for a [`CanvasAction`], or `None` for the
/// parameterized variants ([`CanvasAction::InsertChar`],
/// [`CanvasAction::Custom`]) that can't be named statically.
pub fn canvas_action_name(action: &CanvasAction) -> Option<&'static str> {
    Some(match action {
        CanvasAction::MoveLeft => "move_left",
        CanvasAction::MoveRight => "move_right",
        CanvasAction::MoveUp => "move_up",
        CanvasAction::MoveDown => "move_down",
        CanvasAction::MoveWordNext => "move_word_next",
        CanvasAction::MoveWordPrev => "move_word_prev",
        CanvasAction::MoveWordEnd => "move_word_end",
        CanvasAction::MoveWordEndPrev => "move_word_end_prev",
        CanvasAction::MoveBigWordNext => "move_big_word_next",
        CanvasAction::MoveBigWordPrev => "move_big_word_prev",
        CanvasAction::MoveBigWordEnd => "move_big_word_end",
        CanvasAction::MoveBigWordEndPrev => "move_big_word_end_prev",
        CanvasAction::MoveLineStart => "move_line_start",
        CanvasAction::MoveLineEnd => "move_line_end",
        CanvasAction::NextField => "next_field",
        CanvasAction::PrevField => "prev_field",
        CanvasAction::MoveFirstLine => "move_first_line",
        CanvasAction::MoveLastLine => "move_last_line",
        CanvasAction::DeleteBackward => "delete_char_backward",
        CanvasAction::DeleteForward => "delete_char_forward",
        CanvasAction::Undo => "undo",
        CanvasAction::Redo => "redo",
        CanvasAction::TriggerSuggestions => "trigger_suggestions",
        CanvasAction::SuggestionUp => "suggestion_up",
        CanvasAction::SuggestionDown => "suggestion_down",
        CanvasAction::SelectSuggestion => "select_suggestion",
        CanvasAction::ExitSuggestions => "exit_suggestions",
        CanvasAction::EnterEditMode => "enter_edit_mode_before",
        CanvasAction::EnterEditModeAfter => "enter_edit_mode_after",
        CanvasAction::ExitEditMode => "exit_edit_mode",
        CanvasAction::EnterHighlightMode => "enter_highlight_mode",
        CanvasAction::EnterHighlightModeLinewise => "enter_highlight_mode_linewise",
        CanvasAction::ExitHighlightMode => "exit_highlight_mode",
        CanvasAction::OpenLineBelow => "open_line_below",
        CanvasAction::OpenLineAbove => "open_line_above",
        CanvasAction::InsertChar(_) | CanvasAction::Custom(_) => return None,
        // `CanvasAction` is `#[non_exhaustive]`; unknown future variants have no
        // stable name yet.
        _ => return None,
    })
}

fn is_suggestion_action(action: &CanvasAction) -> bool {
    matches!(
        action,
        CanvasAction::TriggerSuggestions
            | CanvasAction::SuggestionUp
            | CanvasAction::SuggestionDown
            | CanvasAction::SelectSuggestion
            | CanvasAction::ExitSuggestions
    )
}

/// The canvas actions that can be rebound, as [`BindableActionInfo`] lifted into
/// the application action type `A`. Suggestion actions are reported as bindable
/// in the text modes (`ins`/`sel`); the rest in all canvas modes.
pub fn canvas_bindable_actions<A>() -> Vec<BindableActionInfo<A>>
where
    A: From<CanvasAction>,
{
    let mut actions = CanvasAction::movement_actions();
    actions.extend([CanvasAction::DeleteBackward, CanvasAction::DeleteForward]);
    actions.extend([CanvasAction::Undo, CanvasAction::Redo]);
    actions.extend(CanvasAction::suggestions_actions());
    actions.extend([
        CanvasAction::EnterEditMode,
        CanvasAction::EnterEditModeAfter,
        CanvasAction::ExitEditMode,
        CanvasAction::EnterHighlightMode,
        CanvasAction::EnterHighlightModeLinewise,
        CanvasAction::ExitHighlightMode,
        CanvasAction::OpenLineBelow,
        CanvasAction::OpenLineAbove,
    ]);

    actions
        .into_iter()
        .filter_map(|action| {
            let name = canvas_action_name(&action)?;
            let modes = if is_suggestion_action(&action) {
                CANVAS_SUGGESTION_MODES
            } else {
                CANVAS_EDIT_MODES
            };
            Some(BindableActionInfo {
                description: action.description(),
                action: A::from(action),
                name,
                modes,
            })
        })
        .collect()
}

fn key_strokes_to_chords(sequence: &[KeyStroke]) -> Vec<KeyChord> {
    sequence
        .iter()
        .map(|stroke| KeyChord::new(stroke.code, stroke.modifiers))
        .collect()
}

/// The canvas editing defaults for `preset` as a source-tagged catalog, *without*
/// requiring them to be mirrored into the global keymap. This lets a UI show
/// "canvas binds `u` to undo in normal mode" even though the runtime keeps those
/// bindings inside the canvas layer.
///
/// The suggestion-dropdown defaults installed by [`canvas_keybindings`] are
/// reported separately — see [`canvas_suggestion_default_bindings`].
pub fn canvas_default_binding_catalog<A>(
    preset: BuiltinCanvasKeybindingPreset,
) -> BindingCatalog<A>
where
    A: From<CanvasAction>,
{
    let bindings = default_builtin_action_bindings(preset)
        .into_iter()
        .map(|binding| BindingInfo {
            layer: BindingLayer::Canvas,
            mode: mode_for_app_mode(binding.mode).as_str().to_string(),
            sequence: key_strokes_to_chords(&binding.sequence),
            action: A::from(binding.action),
            source: BindingSource::CanvasBuiltin,
        })
        .collect();
    BindingCatalog { bindings }
}

/// The hard-coded suggestion-dropdown bindings ([`bind_suggestion_defaults`]):
/// `Ctrl+Space` trigger, `Ctrl+N`/`Ctrl+P` down/up, `Ctrl+Y` accept, `Ctrl+G`
/// exit — reported for the `ins` and `sel` modes they are installed into.
pub fn canvas_suggestion_default_bindings<A>() -> Vec<BindingInfo<A>>
where
    A: From<CanvasAction>,
{
    let defaults = [
        (KeyCode::Char(' '), CanvasAction::TriggerSuggestions),
        (KeyCode::Char('n'), CanvasAction::SuggestionDown),
        (KeyCode::Char('p'), CanvasAction::SuggestionUp),
        (KeyCode::Char('y'), CanvasAction::SelectSuggestion),
        (KeyCode::Char('g'), CanvasAction::ExitSuggestions),
    ];
    let mut bindings = Vec::new();
    for mode in CANVAS_SUGGESTION_MODES {
        for (code, action) in &defaults {
            bindings.push(BindingInfo {
                layer: BindingLayer::Canvas,
                mode: (*mode).to_string(),
                sequence: vec![KeyChord::new(*code, KeyModifiers::CONTROL)],
                action: A::from(action.clone()),
                source: BindingSource::CanvasBuiltin,
            });
        }
    }
    bindings
}

/// Report keymap/canvas sequence overlaps as [`BindingConflict::CanvasOverlap`].
///
/// When a keymap binding and a canvas binding share a sequence in the same mode,
/// the routing for `context` decides which layer the runtime consults first:
/// `Command`-context keys go to the keymap first ([`CanvasRoutingPrecedence::KeymapFirst`]),
/// `Text`-context keys go to the canvas layer first
/// ([`CanvasRoutingPrecedence::CanvasFirst`]). These are informational — the
/// routing is deterministic — but surface surprises like a config binding that
/// will never fire while editing text.
pub fn analyze_canvas_overlaps<A>(
    keymap_catalog: &BindingCatalog<A>,
    canvas_catalog: &BindingCatalog<CanvasAction>,
    context: InputLayerContext,
) -> Vec<BindingConflict<A>>
where
    A: Clone,
{
    let routing = match context {
        InputLayerContext::Command => CanvasRoutingPrecedence::KeymapFirst,
        InputLayerContext::Text => CanvasRoutingPrecedence::CanvasFirst,
    };

    let mut conflicts = Vec::new();
    for keymap_binding in &keymap_catalog.bindings {
        if keymap_binding.layer != BindingLayer::Keymap {
            continue;
        }
        for canvas_binding in &canvas_catalog.bindings {
            if canvas_binding.mode == keymap_binding.mode
                && canvas_binding.sequence == keymap_binding.sequence
            {
                conflicts.push(BindingConflict::CanvasOverlap {
                    mode: keymap_binding.mode.clone(),
                    sequence: keymap_binding.sequence.clone(),
                    keymap_action: keymap_binding.action.clone(),
                    canvas_action: canvas_binding.action.clone(),
                    routing,
                });
            }
        }
    }
    conflicts
}

#[cfg(test)]
mod report_tests {
    use super::*;
    use crate::input::InputRegistry;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum AppAction {
        Canvas(CanvasAction),
    }

    impl From<CanvasAction> for AppAction {
        fn from(action: CanvasAction) -> Self {
            AppAction::Canvas(action)
        }
    }

    #[test]
    fn default_catalog_carries_canvas_layer_and_source() {
        let catalog: BindingCatalog<AppAction> =
            canvas_default_binding_catalog(BuiltinCanvasKeybindingPreset::Vim);
        assert!(!catalog.bindings.is_empty());
        assert!(catalog.bindings.iter().all(|b| {
            b.layer == BindingLayer::Canvas && b.source == BindingSource::CanvasBuiltin
        }));
    }

    #[test]
    fn suggestion_defaults_cover_ins_and_sel() {
        let bindings: Vec<BindingInfo<AppAction>> = canvas_suggestion_default_bindings();
        assert_eq!(bindings.len(), 10);
        assert!(bindings.iter().any(|b| b.mode == "ins"));
        assert!(bindings.iter().any(|b| b.mode == "sel"));
    }

    #[test]
    fn bindable_actions_have_names() {
        let actions: Vec<BindableActionInfo<AppAction>> = canvas_bindable_actions();
        assert!(actions
            .iter()
            .any(|a| a.name == "suggestion_down" && a.modes == CANVAS_SUGGESTION_MODES));
        assert!(actions.iter().all(|a| !a.name.is_empty()));
    }

    #[test]
    fn overlap_routing_depends_on_context() {
        let mut registry = InputRegistry::<AppAction>::empty();
        // Bind `u` in `nor` in the keymap; canvas vim also binds `u` -> undo.
        registry
            .map_mut("nor")
            .bind(vec![KeyChord::new(KeyCode::Char('u'), KeyModifiers::empty())],
                AppAction::Canvas(CanvasAction::Undo));
        let keymap_catalog = BindingCatalog::from_registry(&registry, BindingSource::Config);
        let canvas_catalog: BindingCatalog<CanvasAction> =
            canvas_default_binding_catalog(BuiltinCanvasKeybindingPreset::Vim);

        let command =
            analyze_canvas_overlaps(&keymap_catalog, &canvas_catalog, InputLayerContext::Command);
        assert!(command.iter().any(|c| matches!(
            c,
            BindingConflict::CanvasOverlap {
                routing: CanvasRoutingPrecedence::KeymapFirst,
                ..
            }
        )));

        let text =
            analyze_canvas_overlaps(&keymap_catalog, &canvas_catalog, InputLayerContext::Text);
        assert!(text.iter().any(|c| matches!(
            c,
            BindingConflict::CanvasOverlap {
                routing: CanvasRoutingPrecedence::CanvasFirst,
                ..
            }
        )));
    }
}
