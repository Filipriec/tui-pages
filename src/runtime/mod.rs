use crate::command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
use crate::focus::{FocusController, FocusIntent, FocusManager, FocusTarget, FocusWrap};
use crate::input::{parse_binding, InputHint, InputPipeline, InputRegistry, KeyChord, KeyMap};
use crate::navigation::{BufferState, PaneSplit};
use crossterm::event::KeyEvent;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;

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
/// type App = TuiPages<View, Action, State, PageFn<View, State>, Handler>;
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
/// type App = TuiPages<View, Action, State, PageFn<View, State, Overlay>, Handler, Overlay>;
/// // this alias names each once
/// type App = TuiApp<View, Action, State, Handler, Overlay>;
/// ```
///
/// `O` (overlay) and `M` (modal payload) default to `()`, so an app with
/// neither writes `TuiApp<View, Action, State, Handler>`. Build it with
/// [`TuiPages::builder`] + [`page_fn`](TuiPagesBuilder::page_fn); the resulting
/// type *is* a `TuiApp`, so `fn build() -> App` lines up with no extra effort.
pub type TuiApp<V, A, S, Handler, O = (), M = ()> =
    TuiPages<V, A, S, PageFn<V, S, O>, Handler, O, M>;

pub trait PageProvider<V, S, O = ()> {
    fn page_spec(&self, view: &V, state: &S, focus: Option<&FocusTarget<O>>) -> PageSpec<O>;
}

impl<V, S, O, F> PageProvider<V, S, O> for F
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

pub trait TuiActionHandler<V, A, S, O = (), M = ()> {
    type Error;

    fn handle_action(
        &mut self,
        action: A,
        ctx: ActionContext<V, O>,
        state: &mut S,
    ) -> Result<ActionOutcome<V, O, M>, Self::Error>;

    fn handle_text(
        &mut self,
        _chord: KeyChord,
        _ctx: ActionContext<V, O>,
        _state: &mut S,
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

#[derive(Debug, Clone)]
pub struct TuiPages<V, A, S, Pages = (), Handler = (), O = (), M = ()> {
    pub input: InputPipeline<A>,
    pub commands: CommandResolver<A>,
    pub focus: FocusManager<O, M>,
    pub buffer: BufferState<V>,
    pages: Pages,
    handler: Handler,
    fallback_view: V,
    pub(crate) text_input_mapper: Option<fn(KeyChord) -> Option<A>>,
    _state: PhantomData<S>,
}

impl<V, A, S, O, M> TuiPages<V, A, S, (), (), O, M>
where
    V: Clone + PartialEq,
{
    pub fn builder(initial_view: V) -> TuiPagesBuilder<V, A, S, O, M, (), ()> {
        TuiPagesBuilder::new(initial_view)
    }
}

impl<V, A, S, Pages, Handler, O, M> TuiPages<V, A, S, Pages, Handler, O, M>
where
    V: Clone + PartialEq,
    A: Clone,
    O: Clone + PartialEq,
    Pages: PageProvider<V, S, O>,
    Handler: TuiActionHandler<V, A, S, O, M>,
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

    pub fn refresh_page(&mut self, state: &S) {
        let spec = self.current_page_spec(state);
        self.sync_focus_to_spec(spec);
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

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error> {
        let spec = self.current_page_spec(state);
        let modes = spec.modes.clone();
        let accepts_text_input = spec.accepts_text_input;
        self.sync_focus_to_spec(spec);

        let focus_accepts_mapped_text = accepts_text_input
            && self
                .focus
                .current()
                .as_ref()
                .map(FocusTarget::is_canvas)
                .unwrap_or(false);
        let response = match self.input.process(key, &modes, accepts_text_input) {
            crate::input::PipelineResponse::Type(chord) if focus_accepts_mapped_text => self
                .text_input_mapper
                .and_then(|mapper| mapper(chord))
                .map(crate::input::PipelineResponse::Execute)
                .unwrap_or(crate::input::PipelineResponse::Type(chord)),
            response => response,
        };
        match response {
            crate::input::PipelineResponse::Execute(action) => {
                let quit_requested = self.dispatch_action(action, state)?;
                Ok(TuiPagesOutput::new(
                    TuiPagesStatus::ActionHandled,
                    quit_requested,
                ))
            }
            crate::input::PipelineResponse::Type(chord) => {
                let quit_requested = self.dispatch_text(chord, state)?;
                Ok(TuiPagesOutput::new(
                    TuiPagesStatus::TextHandled,
                    quit_requested,
                ))
            }
            crate::input::PipelineResponse::Wait(hints) => {
                Ok(TuiPagesOutput::new(TuiPagesStatus::Waiting(hints), false))
            }
            crate::input::PipelineResponse::Cancel => {
                Ok(TuiPagesOutput::new(TuiPagesStatus::Cancelled, false))
            }
        }
    }

    pub fn submit_command(
        &mut self,
        input: &str,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error> {
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

    pub fn apply_effect(&mut self, effect: TuiEffect<V, O, M>, state: &S) -> bool {
        match effect {
            TuiEffect::None => false,
            TuiEffect::Focus(intent) => {
                self.focus.apply_focus_intent(intent);
                false
            }
            TuiEffect::Navigate(view) => {
                self.buffer.update_history(view);
                self.refresh_page(state);
                false
            }
            TuiEffect::NextBuffer => {
                self.switch_buffer(true, state);
                false
            }
            TuiEffect::PreviousBuffer => {
                self.switch_buffer(false, state);
                false
            }
            TuiEffect::CloseBuffer => {
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

    fn current_page_spec(&self, state: &S) -> PageSpec<O> {
        let view = self.current_view();
        let focus = self.focus.current();
        self.pages.page_spec(view, state, focus.as_ref())
    }

    fn dispatch_action(
        &mut self,
        action: A,
        state: &mut S,
    ) -> TuiPagesResult<bool, Handler::Error> {
        let ctx = ActionContext {
            current_view: self.current_view().clone(),
            focus: self.focus.current(),
            has_overlay: self.focus.has_overlay(),
        };
        let outcome = self
            .handler
            .handle_action(action, ctx, state)
            .map_err(TuiPagesError::Handler)?;
        Ok(self.apply_outcome(outcome, state))
    }

    fn dispatch_text(
        &mut self,
        chord: KeyChord,
        state: &mut S,
    ) -> TuiPagesResult<bool, Handler::Error> {
        let ctx = ActionContext {
            current_view: self.current_view().clone(),
            focus: self.focus.current(),
            has_overlay: self.focus.has_overlay(),
        };
        let outcome = self
            .handler
            .handle_text(chord, ctx, state)
            .map_err(TuiPagesError::Handler)?;
        Ok(self.apply_outcome(outcome, state))
    }

    fn apply_outcome(&mut self, outcome: ActionOutcome<V, O, M>, state: &S) -> bool {
        let mut quit_requested = false;
        for effect in outcome.effects {
            quit_requested |= self.apply_effect(effect, state);
        }
        quit_requested
    }

    fn switch_buffer(&mut self, forward: bool, state: &S) {
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

#[derive(Debug, Clone)]
pub struct TuiPagesBuilder<V, A, S, O = (), M = (), Pages = (), Handler = ()> {
    initial_view: V,
    fallback_view: Option<V>,
    pub(crate) input_registry: InputRegistry<A>,
    command_registry: CommandRegistry<A>,
    input_timeout_ms: u64,
    command_timeout_ms: u64,
    focus_wrap: FocusWrap,
    pub(crate) text_input_mapper: Option<fn(KeyChord) -> Option<A>>,
    pages: Pages,
    handler: Handler,
    _state: PhantomData<S>,
    _overlay: PhantomData<O>,
    _modal: PhantomData<M>,
}

impl<V, A, S, O, M> TuiPagesBuilder<V, A, S, O, M, (), ()> {
    pub fn new(initial_view: V) -> Self {
        Self {
            initial_view,
            fallback_view: None,
            input_registry: InputRegistry::empty(),
            command_registry: CommandRegistry::new(),
            input_timeout_ms: 1000,
            command_timeout_ms: 1000,
            focus_wrap: FocusWrap::default(),
            text_input_mapper: None,
            pages: (),
            handler: (),
            _state: PhantomData,
            _overlay: PhantomData,
            _modal: PhantomData,
        }
    }
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler> {
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
    ) -> TuiPagesBuilder<V, A, S, O, M, NextPages, Handler> {
        TuiPagesBuilder {
            initial_view: self.initial_view,
            fallback_view: self.fallback_view,
            input_registry: self.input_registry,
            command_registry: self.command_registry,
            input_timeout_ms: self.input_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            focus_wrap: self.focus_wrap,
            text_input_mapper: self.text_input_mapper,
            pages,
            handler: self.handler,
            _state: PhantomData,
            _overlay: PhantomData,
            _modal: PhantomData,
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
    pub fn page_fn(
        self,
        page_fn: PageFn<V, S, O>,
    ) -> TuiPagesBuilder<V, A, S, O, M, PageFn<V, S, O>, Handler> {
        self.pages(page_fn)
    }

    pub fn handler<NextHandler>(
        self,
        handler: NextHandler,
    ) -> TuiPagesBuilder<V, A, S, O, M, Pages, NextHandler> {
        TuiPagesBuilder {
            initial_view: self.initial_view,
            fallback_view: self.fallback_view,
            input_registry: self.input_registry,
            command_registry: self.command_registry,
            input_timeout_ms: self.input_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            focus_wrap: self.focus_wrap,
            text_input_mapper: self.text_input_mapper,
            pages: self.pages,
            handler,
            _state: PhantomData,
            _overlay: PhantomData,
            _modal: PhantomData,
        }
    }

    /// Set how focus navigation behaves at the ends of a list — clamp (default)
    /// or wrap-around. Applies to page focus and modal items.
    pub fn focus_wrap(mut self, wrap: FocusWrap) -> Self {
        self.focus_wrap = wrap;
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

    pub fn build(self) -> TuiPages<V, A, S, Pages, Handler, O, M>
    where
        V: Clone + PartialEq,
        Pages: PageProvider<V, S, O>,
        Handler: TuiActionHandler<V, A, S, O, M>,
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
            text_input_mapper: self.text_input_mapper,
            _state: PhantomData,
        }
    }
}
