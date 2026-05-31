//! Integration helpers for using the `canvas` crate with `tui-pages`.
//!
//! Enable the `canvas` feature and make your application action type implement
//! `From<CanvasAction>`. Then `.canvas_defaults()` on the builder installs the
//! standard canvas keymaps and typed-character routing, while
//! [`PageSpec::canvas_editor`] keeps the active mode stack in sync with the
//! editor.

use crate::focus::FocusIntent;
use crate::input::{KeyChord, KeyMap};
use crate::runtime::{modes, ModeId, PageSpec, TuiPagesBuilder};
use crossterm::event::{KeyCode, KeyModifiers};

pub use ::canvas::{
    AppMode, CanvasAction, DataProvider, FormEditor,
};
pub use ::canvas::integration::focus_handoff::{
    BoundaryExit, HostActionOutcome, execute_action_for_host,
};

#[derive(Debug, Clone)]
pub enum CanvasDispatchOutcome<O = (), M = ()> {
    Applied(::canvas::ActionResult),
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
    match execute_action_for_host(editor, action) {
        HostActionOutcome::Applied(result) => CanvasDispatchOutcome::Applied(result),
        HostActionOutcome::ExitCanvas(boundary) => {
            CanvasDispatchOutcome::Focus(focus_intent_for_boundary(boundary))
        }
    }
}

pub fn bind_default_keymaps<A>(normal: &mut KeyMap<A>, insert: &mut KeyMap<A>, select: &mut KeyMap<A>)
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
    pub fn canvas_defaults(mut self) -> Self {
        bind_normal_defaults(self.input_registry.map_mut(modes::NORMAL.as_str()));
        bind_insert_defaults(self.input_registry.map_mut(modes::INSERT.as_str()));
        bind_select_defaults(self.input_registry.map_mut(modes::SELECT.as_str()));
        self.text_input_mapper = Some(text_chord_to_action::<A>);
        self
    }

    pub fn canvas_keymaps(mut self) -> Self {
        bind_normal_defaults(self.input_registry.map_mut(modes::NORMAL.as_str()));
        bind_insert_defaults(self.input_registry.map_mut(modes::INSERT.as_str()));
        bind_select_defaults(self.input_registry.map_mut(modes::SELECT.as_str()));
        self
    }

    pub fn canvas_text_input(mut self) -> Self {
        self.text_input_mapper = Some(text_chord_to_action::<A>);
        self
    }
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
