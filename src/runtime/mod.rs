use crate::command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
use crate::focus::{FocusController, FocusIntent, FocusManager, FocusTarget};
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

pub mod modes {
    use super::ModeId;

    pub const GENERAL: ModeId = ModeId::borrowed("general");
    pub const NORMAL: ModeId = ModeId::borrowed("nor");
    pub const INSERT: ModeId = ModeId::borrowed("ins");
    pub const SELECT: ModeId = ModeId::borrowed("sel");
    pub const PALETTE: ModeId = ModeId::borrowed("palette");
    pub const PICKER: ModeId = ModeId::borrowed("picker");
    pub const COMMAND: ModeId = ModeId::borrowed("command");
    pub const COMMON: ModeId = ModeId::borrowed("common");
    pub const GLOBAL: ModeId = ModeId::borrowed("global");
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSpec {
    pub focus_targets: Vec<FocusTarget>,
    pub modes: Vec<ModeId>,
    pub accepts_text_input: bool,
}

impl Default for PageSpec {
    fn default() -> Self {
        Self {
            focus_targets: Vec::new(),
            modes: vec![modes::GENERAL, modes::GLOBAL],
            accepts_text_input: false,
        }
    }
}

impl PageSpec {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn focus_targets(mut self, targets: Vec<FocusTarget>) -> Self {
        self.focus_targets = targets;
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

pub trait PageProvider<V, S> {
    fn page_spec(&self, view: &V, state: &S, focus: Option<&FocusTarget>) -> PageSpec;
}

impl<V, S, F> PageProvider<V, S> for F
where
    F: Fn(&V, &S, Option<&FocusTarget>) -> PageSpec,
{
    fn page_spec(&self, view: &V, state: &S, focus: Option<&FocusTarget>) -> PageSpec {
        self(view, state, focus)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiEffect<V, D = (), P = ()> {
    None,
    Focus(FocusIntent<D, P>),
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
pub struct ActionOutcome<V, D = (), P = ()> {
    pub effects: Vec<TuiEffect<V, D, P>>,
}

impl<V, D, P> Default for ActionOutcome<V, D, P> {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
        }
    }
}

impl<V, D, P> ActionOutcome<V, D, P> {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn effect(effect: TuiEffect<V, D, P>) -> Self {
        Self {
            effects: vec![effect],
        }
    }

    pub fn effects(effects: impl IntoIterator<Item = TuiEffect<V, D, P>>) -> Self {
        Self {
            effects: effects.into_iter().collect(),
        }
    }

    pub fn push(&mut self, effect: TuiEffect<V, D, P>) {
        self.effects.push(effect);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionContext<V> {
    pub current_view: V,
    pub focus: Option<FocusTarget>,
    pub has_overlay: bool,
}

pub trait TuiActionHandler<V, A, S, D = (), P = ()> {
    type Error;

    fn handle_action(
        &mut self,
        action: A,
        ctx: ActionContext<V>,
        state: &mut S,
    ) -> Result<ActionOutcome<V, D, P>, Self::Error>;

    fn handle_text(
        &mut self,
        _chord: KeyChord,
        _ctx: ActionContext<V>,
        _state: &mut S,
    ) -> Result<ActionOutcome<V, D, P>, Self::Error> {
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
pub struct TuiPages<V, A, S, Pages = (), Handler = (), D = (), P = ()> {
    pub input: InputPipeline<A>,
    pub commands: CommandResolver<A>,
    pub focus: FocusManager<D, P>,
    pub buffer: BufferState<V>,
    pages: Pages,
    handler: Handler,
    fallback_view: V,
    _state: PhantomData<S>,
}

impl<V, A, S, D, P> TuiPages<V, A, S, (), (), D, P>
where
    V: Clone + PartialEq,
{
    pub fn builder(initial_view: V) -> TuiPagesBuilder<V, A, S, D, P, (), ()> {
        TuiPagesBuilder::new(initial_view)
    }
}

impl<V, A, S, Pages, Handler, D, P> TuiPages<V, A, S, Pages, Handler, D, P>
where
    V: Clone + PartialEq,
    A: Clone,
    Pages: PageProvider<V, S>,
    Handler: TuiActionHandler<V, A, S, D, P>,
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
        if self.focus.targets() != spec.focus_targets.as_slice() {
            self.focus.register_page(spec.focus_targets);
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyEvent,
        state: &mut S,
    ) -> TuiPagesResult<TuiPagesOutput<A>, Handler::Error> {
        let spec = self.current_page_spec(state);
        if self.focus.targets() != spec.focus_targets.as_slice() {
            self.focus.register_page(spec.focus_targets.clone());
        }

        let response = self.input.process(key, &spec.modes, spec.accepts_text_input);
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
                Ok(TuiPagesOutput::new(TuiPagesStatus::TextHandled, quit_requested))
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

    pub fn apply_effect(&mut self, effect: TuiEffect<V, D, P>, state: &S) -> bool {
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
                self.buffer.focus_next_pane();
                self.refresh_page(state);
                false
            }
            TuiEffect::PreviousPane => {
                self.buffer.focus_previous_pane();
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

    fn current_page_spec(&self, state: &S) -> PageSpec {
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

    fn apply_outcome(&mut self, outcome: ActionOutcome<V, D, P>, state: &S) -> bool {
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
        self.buffer.active_index = if forward {
            (self.buffer.active_index + 1) % len
        } else {
            (self.buffer.active_index + len - 1) % len
        };
        self.buffer.sync_active_pane_to_active_buffer();
        self.refresh_page(state);
    }
}

#[derive(Debug, Clone)]
pub struct TuiPagesBuilder<V, A, S, D = (), P = (), Pages = (), Handler = ()> {
    initial_view: V,
    fallback_view: Option<V>,
    input_registry: InputRegistry<A>,
    command_registry: CommandRegistry<A>,
    input_timeout_ms: u64,
    command_timeout_ms: u64,
    pages: Pages,
    handler: Handler,
    _state: PhantomData<S>,
    _dialog: PhantomData<D>,
    _picker: PhantomData<P>,
}

impl<V, A, S, D, P> TuiPagesBuilder<V, A, S, D, P, (), ()> {
    pub fn new(initial_view: V) -> Self {
        Self {
            initial_view,
            fallback_view: None,
            input_registry: InputRegistry::empty(),
            command_registry: CommandRegistry::new(),
            input_timeout_ms: 1000,
            command_timeout_ms: 1000,
            pages: (),
            handler: (),
            _state: PhantomData,
            _dialog: PhantomData,
            _picker: PhantomData,
        }
    }
}

impl<V, A, S, D, P, Pages, Handler> TuiPagesBuilder<V, A, S, D, P, Pages, Handler> {
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
    ) -> TuiPagesBuilder<V, A, S, D, P, NextPages, Handler> {
        TuiPagesBuilder {
            initial_view: self.initial_view,
            fallback_view: self.fallback_view,
            input_registry: self.input_registry,
            command_registry: self.command_registry,
            input_timeout_ms: self.input_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            pages,
            handler: self.handler,
            _state: PhantomData,
            _dialog: PhantomData,
            _picker: PhantomData,
        }
    }

    pub fn handler<NextHandler>(
        self,
        handler: NextHandler,
    ) -> TuiPagesBuilder<V, A, S, D, P, Pages, NextHandler> {
        TuiPagesBuilder {
            initial_view: self.initial_view,
            fallback_view: self.fallback_view,
            input_registry: self.input_registry,
            command_registry: self.command_registry,
            input_timeout_ms: self.input_timeout_ms,
            command_timeout_ms: self.command_timeout_ms,
            pages: self.pages,
            handler,
            _state: PhantomData,
            _dialog: PhantomData,
            _picker: PhantomData,
        }
    }

    pub fn keymap(mut self, mode: impl Into<ModeId>, configure: impl FnOnce(&mut KeyMap<A>)) -> Self {
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

    pub fn build(self) -> TuiPages<V, A, S, Pages, Handler, D, P>
    where
        V: Clone + PartialEq,
        Pages: PageProvider<V, S>,
        Handler: TuiActionHandler<V, A, S, D, P>,
    {
        let fallback_view = self
            .fallback_view
            .unwrap_or_else(|| self.initial_view.clone());

        TuiPages {
            input: InputPipeline::new(self.input_registry, self.input_timeout_ms),
            commands: CommandResolver::new(self.command_registry, self.command_timeout_ms),
            focus: FocusManager::new(),
            buffer: BufferState::new(self.initial_view),
            pages: self.pages,
            handler: self.handler,
            fallback_view,
            _state: PhantomData,
        }
    }
}
