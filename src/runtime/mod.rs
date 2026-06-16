use crate::command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
use crate::focus::{FocusController, FocusIntent, FocusManager, FocusTarget, FocusWrap};
use crate::input::{InputHint, InputPipeline, InputRegistry, KeyChord, KeyMap, parse_binding};
use crate::keybindings::{
    BindingStore, KeybindingConfig, KeybindingConfigError, KeybindingInheritance, KeybindingReport,
    NavigationAction,
};
use crate::navigation::{BufferState, PaneSplit};
use crossterm::event::KeyEvent;
#[cfg(feature = "command-line")]
use ratatui::layout::{Constraint, Layout, Rect};
use std::borrow::Cow;
#[cfg(feature = "canvas")]
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
#[cfg(feature = "canvas")]
use std::rc::Rc;

#[cfg(feature = "canvas")]
#[derive(Debug, Clone)]
pub(crate) struct CanvasKeybindingProfileState {
    pub profile: crate::canvas::CanvasKeybindingProfile,
    pub generation: u64,
}

#[cfg(feature = "canvas")]
impl CanvasKeybindingProfileState {
    pub fn new(profile: crate::canvas::CanvasKeybindingProfile) -> Self {
        Self {
            profile,
            generation: 0,
        }
    }

    pub fn replace(&mut self, profile: crate::canvas::CanvasKeybindingProfile) {
        self.profile = profile;
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn bump(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

#[cfg(feature = "canvas")]
pub(crate) type CanvasKeybindingProfileHandle = Rc<RefCell<CanvasKeybindingProfileState>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ModeId(Cow<'static, str>);

impl ModeId {
    pub const fn borrowed(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }

    pub fn owned(value: impl Into<String>) -> Self {
        Self(Cow::Owned(value.into()))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<str> for ModeId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ModeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for ModeId {
    fn from(value: &'static str) -> Self {
        Self::borrowed(value)
    }
}

impl From<String> for ModeId {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

/// Built-in mode identifiers shipped by the runtime.
///
/// These cover the input states the runtime itself reasons about. A [`ModeId`]
/// is just a string key, so consumers are free to define their own modes for
/// their own components — a picker, a palette, a sidebar — without the library
/// knowing anything about them:
///
/// ```ignore
/// const PICKER: ModeId = ModeId::borrowed("picker");
///
/// builder
///     .bind(PICKER, "j", Action::PickerDown)
///     .bind(PICKER, "k", Action::PickerUp);
///
/// // then activate it for the relevant page/overlay:
/// PageSpec::new().modes(vec![modes::GLOBAL, PICKER])
/// ```
///
/// Nothing in the runtime is hardcoded to a specific component mode; register
/// whatever your UI needs.
pub mod modes {
    use super::ModeId;

    /// Default page-navigation mode (Tab, arrows, Enter on buttons).
    pub const GENERAL: ModeId = ModeId::borrowed("general");
    /// Read-only navigation within form fields.
    pub const NORMAL: ModeId = ModeId::borrowed("nor");
    /// Typing into a text field; plain characters flow to the focused input.
    pub const INSERT: ModeId = ModeId::borrowed("ins");
    /// Text selection / highlighting.
    pub const SELECT: ModeId = ModeId::borrowed("sel");
    /// Command bar (`:`) is open.
    pub const COMMAND: ModeId = ModeId::borrowed("command");
    /// Bindings shared across non-typing modes (active alongside `nor` and `sel`).
    pub const COMMON: ModeId = ModeId::borrowed("common");
    /// Always active, regardless of the current mode.
    pub const GLOBAL: ModeId = ModeId::borrowed("global");
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageSpec<O = ()> {
    pub focus_targets: Vec<FocusTarget<O>>,
    /// `(section_id, item_count)` for sections the runtime may enter on its
    /// own. Populated by [`PageSpec::focus`] from
    /// [`PageFocusBuilder::section_with_items`](crate::PageFocusBuilder::section_with_items);
    /// empty when sections are registered count-less.
    pub(crate) section_items: Vec<(usize, usize)>,
    pub modes: Vec<ModeId>,
    pub accepts_text_input: bool,
}

impl<O> Default for PageSpec<O> {
    fn default() -> Self {
        Self {
            focus_targets: Vec::new(),
            section_items: Vec::new(),
            modes: vec![modes::GENERAL, modes::GLOBAL],
            accepts_text_input: false,
        }
    }
}

impl<O> PageSpec<O> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn focus_targets(mut self, targets: Vec<FocusTarget<O>>) -> Self {
        self.focus_targets = targets;
        self
    }

    /// Set the focus targets *and* section item counts from a builder in one
    /// call. Prefer this over [`focus_targets`](Self::focus_targets) when any
    /// section is declared with
    /// [`section_with_items`](crate::PageFocusBuilder::section_with_items), so
    /// the runtime can enter the section on its own.
    pub fn focus(mut self, builder: crate::PageFocusBuilder<O>) -> Self {
        let (targets, section_items) = builder.into_parts();
        self.focus_targets = targets;
        self.section_items = section_items;
        self
    }

    pub fn modes(mut self, modes: impl IntoIterator<Item = ModeId>) -> Self {
        self.modes = modes.into_iter().collect();
        self
    }

    pub fn accepts_text_input(mut self, accepts_text_input: bool) -> Self {
        self.accepts_text_input = accepts_text_input;
        self
    }
}

/// A plain function that maps `(view, state, focus)` to a [`PageSpec`].
///
/// Most apps describe their pages with a free function; this alias spells out
/// the signature so a `type App = TuiPages<…>` alias can name the page
/// provider:
///
/// ```ignore
/// type App = TuiPages<View, Action, PageFn<View, State>, Handler>;
/// //                  builder: .page_fn(page_spec)   // coerces for you
/// ```
///
/// Pass the function to [`page_fn`](TuiPagesBuilder::page_fn) rather than
/// [`pages`](TuiPagesBuilder::pages): it pins this pointer type and coerces the
/// fn item at the call site, so the application never writes
/// `page_spec as PageFn<…>`.
pub type PageFn<V, S, O = ()> = fn(&V, &S, Option<&FocusTarget<O>>) -> PageSpec<O>;

/// The common shape of a [`TuiPages`] application: pages described by a plain
/// [`PageFn`].
///
/// [`TuiPages`] carries the page provider as its own type parameter so an
/// advanced caller can plug in any [`PageProvider`]. Almost no one needs that —
/// pages are a free function — and spelling the provider out forces the view
/// and state types to be repeated inside `PageFn<…>`:
///
/// ```ignore
/// // the long form names View / State / Overlay twice
/// type App = TuiPages<View, Action, PageFn<View, State, Overlay>, Handler, Overlay>;
/// // this alias names each once
/// type App = TuiApp<View, Action, State, Handler, Overlay>;
/// ```
///
/// `O` (overlay) and `M` (modal payload) default to `()`, so an app with
/// neither writes `TuiApp<View, Action, State, Handler>`. Build it with
/// [`TuiPages::builder`] + [`page_fn`](TuiPagesBuilder::page_fn); the resulting
/// type *is* a `TuiApp`, so `fn build() -> App` lines up with no extra effort.
pub type TuiApp<V, A, S, Handler, O = (), M = ()> =
    TuiPages<V, A, PageFn<V, S, O>, Handler, O, M>;

pub trait PageProvider<V, S: ?Sized, O = ()> {
    fn page_spec(&self, view: &V, state: &S, focus: Option<&FocusTarget<O>>) -> PageSpec<O>;
}

impl<V, S: ?Sized, O, F> PageProvider<V, S, O> for F
where
    F: Fn(&V, &S, Option<&FocusTarget<O>>) -> PageSpec<O>,
{
    fn page_spec(&self, view: &V, state: &S, focus: Option<&FocusTarget<O>>) -> PageSpec<O> {
        self(view, state, focus)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiEffect<V, O = (), M = ()> {
    None,
    Focus(FocusIntent<O, M>),
    Navigate(V),
    NextBuffer,
    PreviousBuffer,
    CloseBuffer,
    SplitPane(PaneSplit),
    ClosePane,
    NextPane,
    PreviousPane,
    RefreshPage,
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome<V, O = (), M = ()> {
    pub effects: Vec<TuiEffect<V, O, M>>,
}

impl<V, O, M> Default for ActionOutcome<V, O, M> {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
        }
    }
}

impl<V, O, M> ActionOutcome<V, O, M> {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn effect(effect: TuiEffect<V, O, M>) -> Self {
        Self {
            effects: vec![effect],
        }
    }

    pub fn effects(effects: impl IntoIterator<Item = TuiEffect<V, O, M>>) -> Self {
        Self {
            effects: effects.into_iter().collect(),
        }
    }

    pub fn push(&mut self, effect: TuiEffect<V, O, M>) {
        self.effects.push(effect);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContext<V, O = ()> {
    pub current_view: V,
    pub focus: Option<FocusTarget<O>>,
    pub has_overlay: bool,
}

pub struct RuntimeContext<'a, A, O = (), M = ()> {
    pub focus: &'a mut FocusManager<O, M>,
    pub commands: &'a mut CommandResolver<A>,
}

pub trait TuiActionHandler<V, A, S: ?Sized, O = (), M = ()> {
    type Error;

    fn handle_action(
        &mut self,
        action: A,
        ctx: ActionContext<V, O>,
        state: &mut S,
        runtime: RuntimeContext<'_, A, O, M>,
    ) -> Result<ActionOutcome<V, O, M>, Self::Error>;

    fn handle_text(
        &mut self,
        _chord: KeyChord,
        _ctx: ActionContext<V, O>,
        _state: &mut S,
        _runtime: RuntimeContext<'_, A, O, M>,
    ) -> Result<ActionOutcome<V, O, M>, Self::Error> {
        Ok(ActionOutcome::none())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiPagesError<E> {
    Handler(E),
}

impl<E> fmt::Display for TuiPagesError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiPagesError::Handler(error) => write!(f, "handler error: {error}"),
        }
    }
}

impl<E> Error for TuiPagesError<E> where E: Error + 'static {}

impl<E> From<E> for TuiPagesError<E> {
    fn from(error: E) -> Self {
        Self::Handler(error)
    }
}

pub type TuiPagesResult<T, E> = Result<T, TuiPagesError<E>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiPagesStatus<A> {
    ActionHandled,
    TextHandled,
    Waiting(Vec<InputHint<A>>),
    Cancelled,
    CommandIncomplete(Vec<CommandHint>),
    CommandUnknown,
    CommandEmpty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiPagesOutput<A> {
    pub status: TuiPagesStatus<A>,
    pub quit_requested: bool,
}

impl<A> TuiPagesOutput<A> {
    fn new(status: TuiPagesStatus<A>, quit_requested: bool) -> Self {
        Self {
            status,
            quit_requested,
        }
    }
}

pub(crate) struct KeyHookOutcome<V, A, O, M> {
    pub status: TuiPagesStatus<A>,
    pub outcome: ActionOutcome<V, O, M>,
    pub routing: KeyHookRouting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// Both variants are only *constructed* by the canvas key hooks; without the
// `canvas` feature they're still matched in `run_layer` but never built.
#[cfg_attr(not(feature = "canvas"), allow(dead_code))]
pub(crate) enum KeyHookRouting {
    Handled,
    Pending,
}

/// Which input context a focused widget is in. `Command`-context keys are
/// routed to the global keymap first; `Text`-context keys flow to the canvas
/// editing layer first. Exposed so [`crate::canvas::analyze_canvas_overlaps`]
/// can explain which layer wins for an overlapping sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputLayerContext {
    Command,
    Text,
}

#[cfg(feature = "command-line")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandLineAreas {
    pub page: Rect,
    pub command_line: Option<Rect>,
}

#[cfg(feature = "command-line")]
impl CommandLineAreas {
    pub fn split(area: Rect, reserve_command_line: bool) -> Self {
        if !reserve_command_line {
            return Self {
                page: area,
                command_line: None,
            };
        }

        let [page, command_line] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

        Self {
            page,
            command_line: Some(command_line),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum KeyHookKind {
    #[cfg(feature = "canvas")]
    CanvasFormEditor {
        id: usize,
        profile: CanvasKeybindingProfileHandle,
        installed_generation: Option<u64>,
    },
    #[cfg(feature = "canvas")]
    CanvasTextArea {
        focus_index: usize,
        profile: CanvasKeybindingProfileHandle,
        installed_generation: Option<u64>,
    },
    #[cfg(feature = "canvas")]
    CanvasTextInput {
        focus_index: usize,
        profile: CanvasKeybindingProfileHandle,
        installed_generation: Option<u64>,
    },
}

#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "canvas"), allow(dead_code))]
pub(crate) struct KeyHook {
    pub kind: KeyHookKind,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoCanvasHooks;

#[cfg(feature = "canvas")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CanvasHooks;

pub(crate) trait HookDispatch<S: ?Sized, V, A, O, M> {
    fn focused_hook_context(
        hooks: &[KeyHook],
        ctx: &ActionContext<V, O>,
        state: &S,
    ) -> Option<(usize, InputLayerContext)>;

    fn dispatch_hook(
        hook: &mut KeyHook,
        key: KeyEvent,
        ctx: ActionContext<V, O>,
        state: &mut S,
    ) -> Option<KeyHookOutcome<V, A, O, M>>;

    fn paste_hook(
        hook: &mut KeyHook,
        text: &str,
        ctx: ActionContext<V, O>,
        state: &mut S,
    ) -> Option<KeyHookOutcome<V, A, O, M>>;
}

impl<S: ?Sized, V, A, O, M> HookDispatch<S, V, A, O, M> for NoCanvasHooks {
    fn focused_hook_context(
        _hooks: &[KeyHook],
        _ctx: &ActionContext<V, O>,
        _state: &S,
    ) -> Option<(usize, InputLayerContext)> {
        None
    }

    fn dispatch_hook(
        _hook: &mut KeyHook,
        _key: KeyEvent,
        _ctx: ActionContext<V, O>,
        _state: &mut S,
    ) -> Option<KeyHookOutcome<V, A, O, M>> {
        None
    }

    fn paste_hook(
        _hook: &mut KeyHook,
        _text: &str,
        _ctx: ActionContext<V, O>,
        _state: &mut S,
    ) -> Option<KeyHookOutcome<V, A, O, M>> {
        None
    }
}

#[cfg(feature = "canvas")]
impl<S, V, A, O, M> HookDispatch<S, V, A, O, M> for CanvasHooks
where
    S: crate::canvas::CanvasWidgetState + ?Sized,
{
    fn focused_hook_context(
        hooks: &[KeyHook],
        ctx: &ActionContext<V, O>,
        state: &S,
    ) -> Option<(usize, InputLayerContext)> {
        hooks.iter().enumerate().find_map(|(index, hook)| {
            crate::canvas::canvas_hook_context(&hook.kind, ctx, state)
                .map(|context| (index, context))
        })
    }

    fn dispatch_hook(
        hook: &mut KeyHook,
        key: KeyEvent,
        ctx: ActionContext<V, O>,
        state: &mut S,
    ) -> Option<KeyHookOutcome<V, A, O, M>> {
        crate::canvas::dispatch_canvas_key_hook(&mut hook.kind, key, ctx, state)
    }

    fn paste_hook(
        hook: &mut KeyHook,
        text: &str,
        ctx: ActionContext<V, O>,
        state: &mut S,
    ) -> Option<KeyHookOutcome<V, A, O, M>> {
        crate::canvas::dispatch_canvas_paste_hook(&mut hook.kind, text, ctx, state)
    }
}

/// Identifies which input layer the orchestrator is talking to. The global
/// keymap (`self.input`) and each registered canvas [`KeyHook`] are layers; the
/// orchestrator in [`TuiPages::handle_key`] drives them as an ordered stack and
/// remembers which one owns an in-flight multi-key sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerOwner {
    /// The global keymap pipeline (`self.input`).
    Keymap,
    /// The canvas key hook at this index in `key_hooks`.
    Hook(usize),
}

/// The result of offering a key to a single input layer. Unifies the keymap's
/// `PipelineResponse` and a canvas hook's `KeyHookOutcome` so the orchestrator
/// can treat every layer the same way.
pub(crate) enum LayerResult<A> {
    /// The layer began (or continued) a multi-key sequence and now owns
    /// subsequent keys until it resolves. Carries the status to report.
    Pending(TuiPagesStatus<A>),
    /// The layer fully handled the key; nothing else should see it.
    Handled(TuiPagesOutput<A>),
    /// The layer declined the key; try the next layer. Carries the typed chord
    /// when the keymap produced one, so the final text fallback can use it.
    Ignored(Option<KeyChord>),
}

#[derive(Debug, Clone)]
pub struct TuiPages<V, A, Pages = (), Handler = (), O = (), M = (), Hooks = NoCanvasHooks> {
    pub input: InputPipeline<A>,
    pub commands: CommandResolver<A>,
    pub focus: FocusManager<O, M>,
    pub buffer: BufferState<V>,
    pages: Pages,
    handler: Handler,
    fallback_view: V,
    reserve_command_line: bool,
    pub(crate) text_input_mapper: Option<fn(KeyChord) -> Option<A>>,
    pub(crate) key_hooks: Vec<KeyHook>,
    /// The layer that owns the in-flight multi-key sequence, if any. Set when a
    /// layer returns [`LayerResult::Pending`]; subsequent keys route straight to
    /// it until it resolves. `None` means the next key is freshly arbitrated.
    pub(crate) active_owner: Option<LayerOwner>,
    keybinding_store: Option<BindingStore<A>>,
    keybinding_report: Option<KeybindingReport<A>>,
    keybinding_inheritances: Vec<KeybindingInheritance<A>>,
    action_registry: Option<crate::keybindings::ActionRegistry<A>>,
    #[cfg(feature = "canvas")]
    canvas_keybinding_profile: CanvasKeybindingProfileHandle,
    _hooks: PhantomData<Hooks>,
}

impl<V, A, O, M> TuiPages<V, A, (), (), O, M>
where
    V: Clone + PartialEq,
{
    pub fn builder(initial_view: V) -> TuiPagesBuilder<V, A, O, M, (), (), NoCanvasHooks> {
        TuiPagesBuilder::new(initial_view)
    }
}

impl<V, A, Pages, Handler, O, M, Hooks> TuiPages<V, A, Pages, Handler, O, M, Hooks>
where
    V: Clone + PartialEq,
    A: Clone,
    O: Clone + PartialEq,
{
    pub fn current_view(&self) -> &V {
        self.buffer
            .get_active_view()
            .expect("TuiPages buffer always contains at least one view")
    }

    pub fn pages(&self) -> &Pages {
        &self.pages
    }

    pub fn pages_mut(&mut self) -> &mut Pages {
        &mut self.pages
    }

    pub fn handler(&self) -> &Handler {
        &self.handler
    }

    pub fn handler_mut(&mut self) -> &mut Handler {
        &mut self.handler
    }

    pub fn reserve_command_line(&self) -> bool {
        self.reserve_command_line
    }

    #[cfg(feature = "command-line")]
    pub fn render_areas(&self, area: Rect) -> CommandLineAreas {
        CommandLineAreas::split(area, self.reserve_command_line)
    }

    pub fn refresh_page<S: ?Sized>(&mut self, state: &S)
    where
        Pages: PageProvider<V, S, O>,
    {
        let spec = self.current_page_spec(state);
        self.sync_focus_to_spec(spec);
    }

    /// Drop all in-flight input-routing state in one place: the keymap's
    /// partial sequence *and* the sticky [`active_owner`](Self) that a canvas
    /// hook (or the keymap) may hold. Call this whenever the world shifts out
    /// from under a pending sequence — page navigation, buffer switch, or after
    /// remapping bindings at runtime — so a half-typed chord can't resolve
    /// against the new context or be delivered to a layer that no longer owns
    /// the focus.
    pub fn reset_input_routing(&mut self) {
        self.input.reset();
        self.active_owner = None;
    }

    pub fn take_keybinding_report(&mut self) -> Option<KeybindingReport<A>> {
        self.keybinding_report.take()
    }

    pub fn keybinding_store(&self) -> Option<&BindingStore<A>> {
        self.keybinding_store.as_ref()
    }

    /// The key currently bound to `action`, formatted for display (e.g.
    /// `"Ctrl+b"`, or `"g d"` for a chord sequence), or `None` if it is
    /// unbound. The one-liner behind a footer/help hint — "which key does X?".
    ///
    /// If the action is bound to several sequences, the lexicographically first
    /// is returned; use [`keys_for`](Self::keys_for) to get them all.
    ///
    /// ```ignore
    /// let hint = app.key_for(&Action::ToggleSidebar).unwrap_or_else(|| "(unbound)".into());
    /// ```
    pub fn key_for(&self, action: &A) -> Option<String>
    where
        A: PartialEq,
    {
        self.keys_for(action).into_iter().next()
    }

    /// Every key sequence currently bound to `action`, each formatted for
    /// display and sorted for stable output. Use for actions that may carry
    /// more than one binding; for the common single-binding case reach for
    /// [`key_for`](Self::key_for).
    pub fn keys_for(&self, action: &A) -> Vec<String>
    where
        A: PartialEq,
    {
        let mut keys: Vec<String> = self
            .input
            .registry
            .maps
            .values()
            .flat_map(|map| map.bindings_for(action))
            .map(|sequence| {
                sequence
                    .iter()
                    .map(|chord| chord.display_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    fn keybinding_builtin_registry(&self) -> InputRegistry<A>
    where
        A: Clone + PartialEq,
    {
        self.keybinding_store
            .as_ref()
            .map(BindingStore::builtin_registry)
            .unwrap_or_else(|| self.input.registry.clone())
    }

    fn set_keybinding_store_and_registry(
        &mut self,
        store: BindingStore<A>,
        report: KeybindingReport<A>,
    ) where
        A: Clone + PartialEq,
    {
        self.input.registry = store.effective_registry();
        self.keybinding_store = Some(store);
        self.keybinding_report = Some(report);
        self.reset_input_routing();
    }

    pub fn apply_keybindings_toml(
        &mut self,
        source: &str,
    ) -> Result<KeybindingReport<A>, KeybindingConfigError>
    where
        A: Clone + PartialEq + From<NavigationAction>,
    {
        let config = KeybindingConfig::from_toml(source)?;
        let builtin = self.keybinding_builtin_registry();
        let actions = self
            .action_registry
            .clone()
            .unwrap_or_else(crate::keybindings::ActionRegistry::navigation);
        let (store, _, report) = BindingStore::with_user_config_and_inheritances(
            &builtin,
            &config,
            &actions,
            self.keybinding_inheritances.clone(),
        )?;
        #[cfg(feature = "canvas")]
        {
            let profile = config.canvas_profile()?;
            self.canvas_keybinding_profile.borrow_mut().replace(profile);
        }
        self.set_keybinding_store_and_registry(store, report.clone());
        Ok(report)
    }

    /// Serialize the current live keybindings (config + runtime rebinds) to the
    /// unified TOML schema — the inverse of [`Self::apply_keybindings_toml`] and
    /// the builder's `keybindings_toml`. Persisting the string is the caller's
    /// job, e.g. `std::fs::write(path, app.export_keybindings_toml()?)?;`, and a
    /// later launch loads it back via `builder.keybindings_toml(&contents)`.
    pub fn export_keybindings_toml(&self) -> Result<String, KeybindingConfigError>
    where
        A: Clone + PartialEq + From<NavigationAction>,
    {
        let actions = self
            .action_registry
            .clone()
            .unwrap_or_else(crate::keybindings::ActionRegistry::navigation);
        let store = self.keybinding_store.clone().unwrap_or_default();
        #[cfg(feature = "canvas")]
        {
            let profile = self.canvas_keybinding_profile.borrow().profile.clone();
            crate::keybindings::export_to_toml(&store, &actions, &profile)
        }
        #[cfg(not(feature = "canvas"))]
        {
            crate::keybindings::export_to_toml(&store, &actions)
        }
    }

    pub fn rebind_keymap(
        &mut self,
        mode: impl Into<String>,
        sequence: &str,
        action: A,
    ) -> Result<KeybindingReport<A>, KeybindingConfigError>
    where
        A: Clone + PartialEq,
    {
        let mode = mode.into();
        let sequence =
            crate::input::try_parse_binding(sequence).map_err(KeybindingConfigError::KeyBinding)?;
        let mut store = self.keybinding_store.clone().unwrap_or_else(|| {
            let mut store = BindingStore::default();
            store.builtin_keymap = crate::input::BindingCatalog::from_registry(
                &self.input.registry,
                crate::input::BindingSource::Builtin,
            );
            store
        });
        store
            .runtime_keymap
            .bindings
            .retain(|binding| !(binding.mode == mode && binding.action == action));
        store.runtime_keymap.push(crate::input::BindingInfo {
            layer: crate::input::BindingLayer::Keymap,
            mode,
            sequence,
            action,
            source: crate::input::BindingSource::Runtime,
        });
        let report = store.report(&["global", "general", "nor", "ins", "sel"]);
        self.set_keybinding_store_and_registry(store, report.clone());
        Ok(report)
    }

    pub fn reset_keybindings_to_defaults(&mut self)
    where
        A: Clone + PartialEq,
    {
        if let Some(mut store) = self.keybinding_store.clone() {
            store.user_keymap.bindings.clear();
            store.runtime_keymap.bindings.clear();
            #[cfg(feature = "canvas")]
            {
                store.user_canvas.bindings.clear();
                store.runtime_canvas.bindings.clear();
                if let Some(first) = store.builtin_canvas.bindings.first() {
                    let preset = match first.source {
                        crate::input::BindingSource::CanvasBuiltin => {
                            self.canvas_keybinding_profile.borrow().profile.preset()
                        }
                        _ => crate::canvas::BuiltinCanvasKeybindingPreset::Vim,
                    };
                    self.canvas_keybinding_profile
                        .borrow_mut()
                        .replace(preset.profile());
                }
            }
            let report = store.report(&["global", "general", "nor", "ins", "sel"]);
            self.set_keybinding_store_and_registry(store, report);
        } else {
            self.reset_input_routing();
        }
    }

    #[cfg(feature = "canvas")]
    pub fn rebind_canvas(
        &mut self,
        mode: crate::canvas::AppMode,
        action_name: &str,
        sequences: Vec<String>,
    ) -> Result<KeybindingReport<A>, KeybindingConfigError>
    where
        A: Clone + PartialEq,
    {
        let action = crate::canvas::CanvasKeyAction::from_name(action_name);
        if matches!(action, crate::canvas::CanvasKeyAction::Unknown(_)) {
            return Err(KeybindingConfigError::CanvasAction {
                action: action_name.to_string(),
            });
        }
        // Parse every sequence with tui-pages' parser up front, before mutating
        // anything, so a parse error can't leave the live profile half-changed.
        let parsed_sequences = sequences
            .iter()
            .map(|sequence| crate::input::try_parse_binding(sequence))
            .collect::<Result<Vec<_>, _>>()
            .map_err(KeybindingConfigError::KeyBinding)?;
        {
            let mut profile = self.canvas_keybinding_profile.borrow_mut();
            profile
                .profile
                .remap_action(mode, action.clone(), sequences.clone())
                .map_err(KeybindingConfigError::Canvas)?;
            profile.bump();
        }

        let mut store = self.keybinding_store.clone().unwrap_or_else(|| {
            let mut store = BindingStore::default();
            store.builtin_keymap = crate::input::BindingCatalog::from_registry(
                &self.input.registry,
                crate::input::BindingSource::Builtin,
            );
            store.builtin_canvas = crate::canvas::canvas_default_binding_catalog(
                self.canvas_keybinding_profile.borrow().profile.preset(),
            );
            store
        });
        store.runtime_canvas.bindings.retain(|binding| {
            !(binding.mode == crate::canvas::mode_for_app_mode(mode).as_str()
                && crate::canvas::canvas_action_name(&binding.action) == Some(action_name))
        });
        if let Some(canvas_action) = action.to_canvas_action() {
            for sequence in parsed_sequences {
                store.runtime_canvas.push(crate::input::BindingInfo {
                    layer: crate::input::BindingLayer::Canvas,
                    mode: crate::canvas::mode_for_app_mode(mode).as_str().to_string(),
                    sequence,
                    action: canvas_action.clone(),
                    source: crate::input::BindingSource::Runtime,
                });
            }
        }
        let report = store.report(&["global", "general", "nor", "ins", "sel"]);
        self.keybinding_store = Some(store);
        self.keybinding_report = Some(report.clone());
        self.reset_input_routing();
        Ok(report)
    }

    /// Register a page spec's focus targets and section item counts. Targets
    /// are only re-registered when they actually change (so focus position is
    /// preserved across redraws), but the section item counts are always
    /// refreshed — a list may grow or shrink while its `Section` target stays
    /// the same.
    fn sync_focus_to_spec(&mut self, spec: PageSpec<O>) {
        let PageSpec {
            focus_targets,
            section_items,
            ..
        } = spec;
        if self.focus.targets() != focus_targets.as_slice() {
            self.focus.register_page(focus_targets);
        }
        self.focus.set_section_items(section_items);
    }

    fn handle_key_inner<S: ?Sized>(
        &mut self,
        key: KeyEvent,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        let spec = self.current_page_spec(state);
        let modes = spec.modes.clone();
        let page_accepts_text_input = spec.accepts_text_input;
        self.sync_focus_to_spec(spec);

        let ctx = ActionContext {
            current_view: self.current_view().clone(),
            focus: self.focus.current(),
            has_overlay: self.focus.has_overlay(),
        };
        let focused_hook = self.focused_hook_context(&ctx, state);
        let focused_canvas_accepts_text =
            matches!(focused_hook, Some((_, InputLayerContext::Text)));
        let accepts_text_input = page_accepts_text_input || focused_canvas_accepts_text;
        let focus_accepts_mapped_text = focused_canvas_accepts_text
            || (page_accepts_text_input
                && self
                    .focus
                    .current()
                    .as_ref()
                    .map(FocusTarget::is_canvas)
                    .unwrap_or(false));

        // The orchestrator drives the input layers as an ordered stack. The
        // global keymap (`self.input`) and each canvas hook are layers; the
        // first to claim the key wins, and whichever begins a multi-key
        // sequence becomes the sticky `active_owner` for the keys that follow.
        //
        let order = self.layer_order(focused_hook, page_accepts_text_input);

        // A sequence in flight routes its continuation straight to its owner.
        if let Some(owner) = self.active_owner {
            match self.run_layer(
                owner,
                key,
                &ctx,
                &modes,
                accepts_text_input,
                focus_accepts_mapped_text,
                state,
            )? {
                LayerResult::Pending(status) => return Ok(TuiPagesOutput::new(status, false)),
                LayerResult::Handled(output) => {
                    self.active_owner = None;
                    return Ok(output);
                }
                // The owner unexpectedly let go of the key; clear it and fall
                // through to a fresh arbitration below.
                LayerResult::Ignored(_) => self.active_owner = None,
            }
        }

        let mut text_chord = None;
        for owner in order {
            match self.run_layer(
                owner,
                key,
                &ctx,
                &modes,
                accepts_text_input,
                focus_accepts_mapped_text,
                state,
            )? {
                LayerResult::Pending(status) => {
                    self.active_owner = Some(owner);
                    return Ok(TuiPagesOutput::new(status, false));
                }
                LayerResult::Handled(output) => return Ok(output),
                LayerResult::Ignored(chord) => {
                    if chord.is_some() {
                        text_chord = chord;
                    }
                }
            }
        }

        // No layer claimed the key: it is plain text for the focused widget.
        let chord = text_chord.unwrap_or_else(|| KeyChord::from_event(&key));
        let quit_requested = self.dispatch_text(chord, state)?;
        Ok(TuiPagesOutput::new(
            TuiPagesStatus::TextHandled,
            quit_requested,
        ))
    }

    fn focused_hook_context<S: ?Sized>(
        &self,
        ctx: &ActionContext<V, O>,
        state: &S,
    ) -> Option<(usize, InputLayerContext)>
    where
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        Hooks::focused_hook_context(&self.key_hooks, ctx, state)
    }

    fn layer_order(
        &self,
        focused_hook: Option<(usize, InputLayerContext)>,
        page_accepts_text_input: bool,
    ) -> Vec<LayerOwner> {
        let mut order = Vec::with_capacity(self.key_hooks.len() + 1);
        let focused_index = focused_hook.map(|(index, _)| index);
        let text_context =
            page_accepts_text_input || matches!(focused_hook, Some((_, InputLayerContext::Text)));

        if text_context {
            if let Some(index) = focused_index {
                order.push(LayerOwner::Hook(index));
            } else {
                order.extend((0..self.key_hooks.len()).map(LayerOwner::Hook));
            }
            order.push(LayerOwner::Keymap);
        } else {
            order.push(LayerOwner::Keymap);
            if let Some(index) = focused_index {
                order.push(LayerOwner::Hook(index));
            }
        }

        let remaining = (0..self.key_hooks.len())
            .map(LayerOwner::Hook)
            .filter(|owner| !order.contains(owner))
            .collect::<Vec<_>>();
        order.extend(remaining);
        order
    }

    /// Offer a key to a single input layer and normalise its outcome into a
    /// [`LayerResult`] the orchestrator can act on uniformly.
    #[allow(clippy::too_many_arguments)]
    fn run_layer<S: ?Sized>(
        &mut self,
        owner: LayerOwner,
        key: KeyEvent,
        ctx: &ActionContext<V, O>,
        modes: &[ModeId],
        accepts_text_input: bool,
        focus_accepts_mapped_text: bool,
        state: &mut S,
    ) -> TuiPagesResult<LayerResult<A>, Handler::Error>
    where
        Handler: TuiActionHandler<V, A, S, O, M>,
        Pages: PageProvider<V, S, O>,
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        match owner {
            LayerOwner::Hook(index) => {
                let response =
                    Hooks::dispatch_hook(&mut self.key_hooks[index], key, ctx.clone(), state);
                match response {
                    None => Ok(LayerResult::Ignored(None)),
                    Some(KeyHookOutcome {
                        status,
                        outcome,
                        routing,
                    }) => {
                        let quit_requested = self.apply_outcome(outcome, state);
                        if matches!(routing, KeyHookRouting::Pending) {
                            Ok(LayerResult::Pending(status))
                        } else {
                            Ok(LayerResult::Handled(TuiPagesOutput::new(
                                status,
                                quit_requested,
                            )))
                        }
                    }
                }
            }
            LayerOwner::Keymap => {
                let response = match self.input.process(key, modes, accepts_text_input) {
                    crate::input::PipelineResponse::Type(chord) if focus_accepts_mapped_text => {
                        self.text_input_mapper
                            .and_then(|mapper| mapper(chord))
                            .map(crate::input::PipelineResponse::Execute)
                            .unwrap_or(crate::input::PipelineResponse::Type(chord))
                    }
                    response => response,
                };
                match response {
                    crate::input::PipelineResponse::Execute(action) => {
                        let quit_requested = self.dispatch_action(action, state)?;
                        Ok(LayerResult::Handled(TuiPagesOutput::new(
                            TuiPagesStatus::ActionHandled,
                            quit_requested,
                        )))
                    }
                    crate::input::PipelineResponse::Wait(hints) => {
                        Ok(LayerResult::Pending(TuiPagesStatus::Waiting(hints)))
                    }
                    crate::input::PipelineResponse::Cancel => Ok(LayerResult::Handled(
                        TuiPagesOutput::new(TuiPagesStatus::Cancelled, false),
                    )),
                    crate::input::PipelineResponse::Type(chord) => {
                        Ok(LayerResult::Ignored(Some(chord)))
                    }
                }
            }
        }
    }

    /// Route a bracketed-paste payload to the focused canvas widget, if any.
    ///
    /// Mirrors [`handle_key`](Self::handle_key): it walks the registered hooks
    /// and lets the first one whose widget currently holds focus consume the
    /// pasted text. Returns [`TuiPagesStatus::TextHandled`] when a widget took
    /// the paste, and [`TuiPagesStatus::Cancelled`] when nothing was focused to
    /// receive it.
    fn handle_paste_inner<S: ?Sized>(
        &mut self,
        text: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        let spec = self.current_page_spec(state);
        self.sync_focus_to_spec(spec);

        let ctx = ActionContext {
            current_view: self.current_view().clone(),
            focus: self.focus.current(),
            has_overlay: self.focus.has_overlay(),
        };

        let mut paste_response: Option<KeyHookOutcome<V, A, O, M>> = None;
        for hook in &mut self.key_hooks {
            let response = Hooks::paste_hook(hook, text, ctx.clone(), state);
            if let Some(response) = response {
                paste_response = Some(response);
                break;
            }
        }

        if let Some(response) = paste_response {
            let quit_requested = self.apply_outcome(response.outcome, state);
            return Ok(TuiPagesOutput::new(response.status, quit_requested));
        }

        Ok(TuiPagesOutput::new(TuiPagesStatus::Cancelled, false))
    }

    fn submit_command_inner<S: ?Sized>(
        &mut self,
        input: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        match self.commands.process(input) {
            CommandResponse::Execute(action) => {
                let quit_requested = self.dispatch_action(action, state)?;
                Ok(TuiPagesOutput::new(
                    TuiPagesStatus::ActionHandled,
                    quit_requested,
                ))
            }
            CommandResponse::Incomplete(hints) => Ok(TuiPagesOutput::new(
                TuiPagesStatus::CommandIncomplete(hints),
                false,
            )),
            CommandResponse::Unknown => {
                Ok(TuiPagesOutput::new(TuiPagesStatus::CommandUnknown, false))
            }
            CommandResponse::Empty => Ok(TuiPagesOutput::new(TuiPagesStatus::CommandEmpty, false)),
        }
    }

    pub fn apply_effect<S: ?Sized>(
        &mut self,
        effect: TuiEffect<V, O, M>,
        state: &S,
    ) -> bool
    where
        Pages: PageProvider<V, S, O>,
    {
        match effect {
            TuiEffect::None => false,
            TuiEffect::Focus(intent) => {
                self.focus.apply_focus_intent(intent);
                false
            }
            TuiEffect::Navigate(view) => {
                self.reset_input_routing();
                self.buffer.update_history(view);
                self.refresh_page(state);
                false
            }
            TuiEffect::NextBuffer => {
                self.reset_input_routing();
                self.switch_buffer(true, state);
                false
            }
            TuiEffect::PreviousBuffer => {
                self.reset_input_routing();
                self.switch_buffer(false, state);
                false
            }
            TuiEffect::CloseBuffer => {
                self.reset_input_routing();
                self.buffer.close_active_buffer(self.fallback_view.clone());
                self.refresh_page(state);
                false
            }
            TuiEffect::SplitPane(split) => {
                self.buffer.split_active_pane(split);
                false
            }
            TuiEffect::ClosePane => {
                self.buffer.close_active_pane();
                self.refresh_page(state);
                false
            }
            TuiEffect::NextPane => {
                self.buffer.focus_next_pane(self.focus.focus_wrap());
                self.refresh_page(state);
                false
            }
            TuiEffect::PreviousPane => {
                self.buffer.focus_previous_pane(self.focus.focus_wrap());
                self.refresh_page(state);
                false
            }
            TuiEffect::RefreshPage => {
                self.refresh_page(state);
                false
            }
            TuiEffect::Quit => true,
        }
    }

    fn current_page_spec<S: ?Sized>(&self, state: &S) -> PageSpec<O>
    where
        Pages: PageProvider<V, S, O>,
    {
        let view = self.current_view();
        let focus = self.focus.current();
        self.pages.page_spec(view, state, focus.as_ref())
    }

    fn dispatch_action<S: ?Sized>(
        &mut self,
        action: A,
        state: &mut S,
    ) -> TuiPagesResult<bool, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        let ctx = ActionContext {
            current_view: self.current_view().clone(),
            focus: self.focus.current(),
            has_overlay: self.focus.has_overlay(),
        };
        let runtime = RuntimeContext {
            focus: &mut self.focus,
            commands: &mut self.commands,
        };
        let outcome = self
            .handler
            .handle_action(action, ctx, state, runtime)
            .map_err(TuiPagesError::Handler)?;
        Ok(self.apply_outcome(outcome, state))
    }

    fn dispatch_text<S: ?Sized>(
        &mut self,
        chord: KeyChord,
        state: &mut S,
    ) -> TuiPagesResult<bool, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
        Hooks: HookDispatch<S, V, A, O, M>,
    {
        let ctx = ActionContext {
            current_view: self.current_view().clone(),
            focus: self.focus.current(),
            has_overlay: self.focus.has_overlay(),
        };
        let runtime = RuntimeContext {
            focus: &mut self.focus,
            commands: &mut self.commands,
        };
        let outcome = self
            .handler
            .handle_text(chord, ctx, state, runtime)
            .map_err(TuiPagesError::Handler)?;
        Ok(self.apply_outcome(outcome, state))
    }

    fn apply_outcome<S: ?Sized>(
        &mut self,
        outcome: ActionOutcome<V, O, M>,
        state: &S,
    ) -> bool
    where
        Pages: PageProvider<V, S, O>,
    {
        let mut quit_requested = false;
        for effect in outcome.effects {
            quit_requested |= self.apply_effect(effect, state);
        }
        quit_requested
    }

    fn switch_buffer<S: ?Sized>(&mut self, forward: bool, state: &S)
    where
        Pages: PageProvider<V, S, O>,
    {
        if self.buffer.history.len() <= 1 {
            return;
        }

        let len = self.buffer.history.len();
        self.buffer.active_index =
            self.focus
                .focus_wrap()
                .step(self.buffer.active_index, len, forward);
        self.buffer.sync_active_pane_to_active_buffer();
        self.refresh_page(state);
    }
}

impl<V, A, Pages, Handler, O, M> TuiPages<V, A, Pages, Handler, O, M, NoCanvasHooks>
where
    V: Clone + PartialEq,
    A: Clone,
    O: Clone + PartialEq,
{
    pub fn handle_key<S: ?Sized>(
        &mut self,
        key: KeyEvent,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
    {
        self.handle_key_inner(key, state)
    }

    pub fn handle_paste<S: ?Sized>(
        &mut self,
        text: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
    {
        self.handle_paste_inner(text, state)
    }

    pub fn submit_command<S: ?Sized>(
        &mut self,
        input: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
    {
        self.submit_command_inner(input, state)
    }
}

#[cfg(feature = "canvas")]
impl<V, A, Pages, Handler, O, M> TuiPages<V, A, Pages, Handler, O, M, CanvasHooks>
where
    V: Clone + PartialEq,
    A: Clone,
    O: Clone + PartialEq,
{
    pub fn handle_key<S>(
        &mut self,
        key: KeyEvent,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        S: crate::canvas::CanvasWidgetState + ?Sized,
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
    {
        self.handle_key_inner(key, state)
    }

    pub fn handle_paste<S>(
        &mut self,
        text: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        S: crate::canvas::CanvasWidgetState + ?Sized,
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
    {
        self.handle_paste_inner(text, state)
    }

    pub fn submit_command<S>(
        &mut self,
        input: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error>
    where
        S: crate::canvas::CanvasWidgetState + ?Sized,
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
    {
        self.submit_command_inner(input, state)
    }
}

#[derive(Debug, Clone)]
pub struct TuiPagesBuilder<
    V,
    A,
    O = (),
    M = (),
    Pages = (),
    Handler = (),
    Hooks = NoCanvasHooks,
> {
    pub(crate) initial_view: V,
    pub(crate) fallback_view: Option<V>,
    pub(crate) input_registry: InputRegistry<A>,
    pub(crate) command_registry: CommandRegistry<A>,
    pub(crate) input_timeout_ms: u64,
    pub(crate) command_timeout_ms: u64,
    pub(crate) focus_wrap: FocusWrap,
    pub(crate) reserve_command_line: bool,
    pub(crate) text_input_mapper: Option<fn(KeyChord) -> Option<A>>,
    pub(crate) key_hooks: Vec<KeyHook>,
    pub(crate) keybinding_store: Option<BindingStore<A>>,
    pub(crate) keybinding_report: Option<KeybindingReport<A>>,
    pub(crate) keybinding_inheritances: Vec<KeybindingInheritance<A>>,
    pub(crate) action_registry: Option<crate::keybindings::ActionRegistry<A>>,
    #[cfg(feature = "canvas")]
    pub(crate) canvas_keybinding_profile: CanvasKeybindingProfileHandle,
    pub(crate) pages: Pages,
    pub(crate) handler: Handler,
    pub(crate) _overlay: PhantomData<O>,
    pub(crate) _modal: PhantomData<M>,
    pub(crate) _hooks: PhantomData<Hooks>,
}

impl<V, A, O, M> TuiPagesBuilder<V, A, O, M, (), (), NoCanvasHooks> {
    pub fn new(initial_view: V) -> Self {
        Self {
            initial_view,
            fallback_view: None,
            input_registry: InputRegistry::empty(),
            command_registry: CommandRegistry::new(),
            input_timeout_ms: 1000,
            command_timeout_ms: 1000,
            focus_wrap: FocusWrap::default(),
            reserve_command_line: cfg!(feature = "command-line"),
            text_input_mapper: None,
            key_hooks: Vec::new(),
            keybinding_store: None,
            keybinding_report: None,
            keybinding_inheritances: Vec::new(),
            action_registry: None,
            #[cfg(feature = "canvas")]
            canvas_keybinding_profile: Rc::new(RefCell::new(CanvasKeybindingProfileState::new(
                crate::canvas::BuiltinCanvasKeybindingPreset::Vim.profile(),
            ))),
            pages: (),
            handler: (),
            _overlay: PhantomData,
            _modal: PhantomData,
            _hooks: PhantomData,
        }
    }
}

impl<V, A, O, M, Pages, Handler, Hooks>
    TuiPagesBuilder<V, A, O, M, Pages, Handler, Hooks>
{
    /// Supply the table that maps `[keymap.*]` action *names* to the app's
    /// action type `A`, used to load and export keybindings. When unset, the
    /// crate defaults to [`ActionRegistry::navigation`](crate::keybindings::ActionRegistry::navigation).
    /// Build one from `navigation_bindable_actions()` /
    /// `canvas_bindable_actions()` plus the app's own bindable actions.
    pub fn action_registry(mut self, registry: crate::keybindings::ActionRegistry<A>) -> Self {
        self.action_registry = Some(registry);
        self
    }

    pub fn fallback_view(mut self, fallback_view: V) -> Self {
        self.fallback_view = Some(fallback_view);
        self
    }

    pub fn input_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.input_timeout_ms = timeout_ms;
        self
    }

    pub fn command_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.command_timeout_ms = timeout_ms;
        self
    }

    pub fn pages<NextPages>(
        self,
        pages: NextPages,
    ) -> TuiPagesBuilder<V, A, O, M, NextPages, Handler, Hooks> {
        TuiPagesBuilder {
            initial_view: self.initial_view,
            fallback_view: self.fallback_view,
            input_registry: self.input_registry,
            command_registry: self.command_registry,
            input_timeout_ms: self.input_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            focus_wrap: self.focus_wrap,
            reserve_command_line: self.reserve_command_line,
            text_input_mapper: self.text_input_mapper,
            key_hooks: self.key_hooks,
            keybinding_store: self.keybinding_store,
            keybinding_report: self.keybinding_report,
            keybinding_inheritances: self.keybinding_inheritances,
            action_registry: self.action_registry,
            #[cfg(feature = "canvas")]
            canvas_keybinding_profile: self.canvas_keybinding_profile,
            pages,
            handler: self.handler,
            _overlay: PhantomData,
            _modal: PhantomData,
            _hooks: PhantomData,
        }
    }

    /// Set the page provider to a plain `fn`, coercing it to [`PageFn`] at the
    /// call site so the application never writes `page_spec as PageFn<…>`.
    ///
    /// `.pages(f)` keeps the fn *item* type, which a `type App = TuiPages<…>`
    /// alias cannot name; this method pins the [`PageFn`] pointer type the
    /// alias uses, so `.page_fn(page_spec)` just works:
    ///
    /// ```ignore
    /// TuiPages::builder(View::Home).page_fn(page_spec).handler(Handler).build()
    /// ```
    pub fn page_fn<S>(
        self,
        page_fn: PageFn<V, S, O>,
    ) -> TuiPagesBuilder<V, A, O, M, PageFn<V, S, O>, Handler, Hooks> {
        self.pages(page_fn)
    }

    pub fn handler<NextHandler>(
        self,
        handler: NextHandler,
    ) -> TuiPagesBuilder<V, A, O, M, Pages, NextHandler, Hooks> {
        TuiPagesBuilder {
            initial_view: self.initial_view,
            fallback_view: self.fallback_view,
            input_registry: self.input_registry,
            command_registry: self.command_registry,
            input_timeout_ms: self.input_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            focus_wrap: self.focus_wrap,
            reserve_command_line: self.reserve_command_line,
            text_input_mapper: self.text_input_mapper,
            key_hooks: self.key_hooks,
            keybinding_store: self.keybinding_store,
            keybinding_report: self.keybinding_report,
            keybinding_inheritances: self.keybinding_inheritances,
            action_registry: self.action_registry,
            #[cfg(feature = "canvas")]
            canvas_keybinding_profile: self.canvas_keybinding_profile,
            pages: self.pages,
            handler,
            _overlay: PhantomData,
            _modal: PhantomData,
            _hooks: PhantomData,
        }
    }

    /// Set how focus navigation behaves at the ends of a list — clamp (default)
    /// or wrap-around. Applies to page focus and modal items.
    pub fn focus_wrap(mut self, wrap: FocusWrap) -> Self {
        self.focus_wrap = wrap;
        self
    }

    pub fn reserve_command_line(mut self, reserve: bool) -> Self {
        self.reserve_command_line = reserve;
        self
    }

    /// Map raw text-input chords into application actions before
    /// [`TuiActionHandler::handle_text`] is called.
    ///
    /// The mapper only runs when the current focus target is a canvas target
    /// and the current [`PageSpec`] accepts text input. This keeps command bars,
    /// palettes, and other text overlays on their normal `handle_text` path.
    pub fn text_input_mapper(mut self, mapper: fn(KeyChord) -> Option<A>) -> Self {
        self.text_input_mapper = Some(mapper);
        self
    }

    pub fn keymap(
        mut self,
        mode: impl Into<ModeId>,
        configure: impl FnOnce(&mut KeyMap<A>),
    ) -> Self {
        let mode = mode.into();
        configure(self.input_registry.map_mut(mode.as_str()));
        self
    }

    pub fn bind(mut self, mode: impl Into<ModeId>, binding: &str, action: A) -> Self {
        let mode = mode.into();
        self.input_registry
            .map_mut(mode.as_str())
            .bind(parse_binding(binding), action);
        self
    }

    /// Replace the built-in keymap layer with a pre-built registry. This is the
    /// app's default bindings, on top of which `[keymap.*]` config overrides and
    /// runtime rebinds layer. Use when the app builds its defaults as a whole
    /// [`InputRegistry`] rather than one `.bind()` at a time; call it before
    /// [`keybindings_toml`](Self::keybindings_toml) so the config layers on top.
    pub fn input_registry(mut self, registry: InputRegistry<A>) -> Self {
        self.input_registry = registry;
        self
    }

    pub fn inherit_keybinding(
        mut self,
        target_mode: impl Into<ModeId>,
        target_action: A,
        source_mode: impl Into<ModeId>,
        source_action: A,
    ) -> Self {
        self.keybinding_inheritances
            .push(KeybindingInheritance::new(
                target_mode,
                target_action,
                source_mode,
                source_action,
            ));
        self
    }

    pub fn command<I, Alias>(
        mut self,
        action_name: impl Into<String>,
        aliases: I,
        action: A,
    ) -> Self
    where
        A: Clone,
        I: IntoIterator<Item = Alias>,
        Alias: Into<String>,
    {
        self.command_registry
            .bind_aliases(action_name, aliases, action);
        self
    }

    pub fn build(self) -> TuiPages<V, A, Pages, Handler, O, M, Hooks>
    where
        V: Clone + PartialEq,
    {
        let fallback_view = self
            .fallback_view
            .unwrap_or_else(|| self.initial_view.clone());

        let mut focus = FocusManager::new();
        focus.set_focus_wrap(self.focus_wrap);

        TuiPages {
            input: InputPipeline::new(self.input_registry, self.input_timeout_ms),
            commands: CommandResolver::new(self.command_registry, self.command_timeout_ms),
            focus,
            buffer: BufferState::new(self.initial_view),
            pages: self.pages,
            handler: self.handler,
            fallback_view,
            reserve_command_line: self.reserve_command_line,
            text_input_mapper: self.text_input_mapper,
            key_hooks: self.key_hooks,
            active_owner: None,
            keybinding_store: self.keybinding_store,
            keybinding_report: self.keybinding_report,
            keybinding_inheritances: self.keybinding_inheritances,
            action_registry: self.action_registry,
            #[cfg(feature = "canvas")]
            canvas_keybinding_profile: self.canvas_keybinding_profile,
            _hooks: PhantomData,
        }
    }
}

impl<V, A, O, M, Pages, Handler, Hooks> TuiPagesBuilder<V, A, O, M, Pages, Handler, Hooks>
where
    A: Clone + PartialEq + From<NavigationAction>,
{
    pub fn keybindings_toml(mut self, source: &str) -> Result<Self, KeybindingConfigError> {
        let config = KeybindingConfig::from_toml(source)?;
        self = self.keybindings_config(config)?;
        Ok(self)
    }

    pub fn keybindings_config(
        mut self,
        config: KeybindingConfig,
    ) -> Result<Self, KeybindingConfigError> {
        let actions = self
            .action_registry
            .clone()
            .unwrap_or_else(crate::keybindings::ActionRegistry::navigation);
        let (store, registry, report) = BindingStore::with_user_config_and_inheritances(
            &self.input_registry,
            &config,
            &actions,
            self.keybinding_inheritances.clone(),
        )?;
        self.input_registry = registry;
        #[cfg(feature = "canvas")]
        {
            let profile = config.canvas_profile()?;
            self.canvas_keybinding_profile.borrow_mut().replace(profile);
        }
        self.keybinding_store = Some(store);
        self.keybinding_report = Some(report);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::NavigationAction;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum View {
        Main,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Action {
        Nav(NavigationAction),
    }

    impl From<NavigationAction> for Action {
        fn from(value: NavigationAction) -> Self {
            Self::Nav(value)
        }
    }

    struct Handler;

    impl TuiActionHandler<View, Action, ()> for Handler {
        type Error = std::convert::Infallible;

        fn handle_action(
            &mut self,
            _action: Action,
            _ctx: ActionContext<View>,
            _state: &mut (),
            _runtime: RuntimeContext<'_, Action>,
        ) -> Result<ActionOutcome<View>, Self::Error> {
            Ok(ActionOutcome::none())
        }
    }

    fn page_spec(_view: &View, _state: &(), _focus: Option<&FocusTarget>) -> PageSpec {
        PageSpec::new()
    }

    #[test]
    fn runtime_rebind_keymap_and_reset_restore_defaults() {
        let mut app = TuiPages::<View, Action>::builder(View::Main)
            .page_fn(page_spec)
            .handler(Handler)
            .bind(modes::GLOBAL, "ctrl+c", Action::Nav(NavigationAction::Quit))
            .build();

        let report = app
            .rebind_keymap("global", "ctrl+q", Action::Nav(NavigationAction::Quit))
            .unwrap();
        assert!(report.notices.is_empty());
        let global = app.input.registry.maps.get("global").unwrap();
        assert!(
            global
                .bindings
                .contains_key(&crate::input::try_parse_binding("ctrl+q").unwrap())
        );
        assert!(
            !global
                .bindings
                .contains_key(&crate::input::try_parse_binding("ctrl+c").unwrap())
        );

        app.reset_keybindings_to_defaults();
        let global = app.input.registry.maps.get("global").unwrap();
        assert!(
            global
                .bindings
                .contains_key(&crate::input::try_parse_binding("ctrl+c").unwrap())
        );
        assert!(
            !global
                .bindings
                .contains_key(&crate::input::try_parse_binding("ctrl+q").unwrap())
        );
    }

    #[test]
    fn export_keybindings_toml_round_trips_and_is_idempotent() {
        // Config override (`focus_next = j` in general) + a runtime rebind
        // (`quit = ctrl+q` in global), exported and reloaded into a fresh app.
        let mut app = TuiPages::<View, Action>::builder(View::Main)
            .page_fn(page_spec)
            .handler(Handler)
            .bind(modes::GLOBAL, "ctrl+c", Action::Nav(NavigationAction::Quit))
            .keybindings_toml("[keymap.general]\nfocus_next = [\"j\"]\n")
            .unwrap()
            .build();
        app.rebind_keymap("global", "ctrl+q", Action::Nav(NavigationAction::Quit))
            .unwrap();

        let exported = app.export_keybindings_toml().unwrap();

        let reloaded = TuiPages::<View, Action>::builder(View::Main)
            .page_fn(page_spec)
            .handler(Handler)
            .bind(modes::GLOBAL, "ctrl+c", Action::Nav(NavigationAction::Quit))
            .keybindings_toml(&exported)
            .unwrap()
            .build();

        let general = reloaded.input.registry.maps.get("general").unwrap();
        assert!(
            general
                .bindings
                .contains_key(&crate::input::try_parse_binding("j").unwrap())
        );
        let global = reloaded.input.registry.maps.get("global").unwrap();
        assert!(
            global
                .bindings
                .contains_key(&crate::input::try_parse_binding("ctrl+q").unwrap())
        );

        // Re-exporting the reloaded state yields the identical document.
        assert_eq!(reloaded.export_keybindings_toml().unwrap(), exported);
    }
}
