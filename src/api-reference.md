# API Reference

Complete reference for all public types and functions.

## Prelude

Import everything commonly needed:

```rust
use tui_pages::prelude::*;
```

This brings in:
- Runtime types: `TuiPages`, `TuiPagesBuilder`, `TuiEffect`, `ActionOutcome`
- Focus types: `FocusManager`, `FocusIntent`, `FocusTarget`, `FocusWrap`
- Input types: `KeyChord`, `InputPipeline`, `InputRegistry`, `parse_binding`
- Mode constants: `modes::*`
- Traits: `TuiActionHandler`, `FocusController`, `PageProvider`

## Runtime Module (`tui_pages::runtime`)

### TuiPages

```rust
pub struct TuiPages<V, A, S, O = (), M = ()>
```

Primary runtime type. Use the builder to construct.

### TuiPagesBuilder

```rust
impl TuiPagesBuilder<V, A, S> {
    pub fn builder(initial_view: V) -> Self;

    // Required
    pub fn pages<P: PageProvider<V, S>>(self, provider: P) -> Self;
    pub fn handler<H: TuiActionHandler<V, A, S>>(self, handler: H) -> Self;

    // Optional configuration
    pub fn focus_wrap(self, wrap: FocusWrap) -> Self;
    pub fn command_timeout(self, ms: u64) -> Self;

    // Key bindings
    pub fn bind(self, mode: ModeId, binding: &str, action: A) -> Self;

    // Commands
    pub fn command(self, name: &str, aliases: impl IntoIterator<Item = &'static str>, action: A) -> Self;

    pub fn build(self) -> TuiPages<V, A, S, O, M>;
}
```

### TuiPages Output

```rust
pub struct TuiPagesOutput<V, O, M> {
    pub status: TuiPagesStatus<V, O, M>,
    pub quit_requested: bool,
    pub effects: Vec<TuiEffect<V, O, M>>,
}

pub enum TuiPagesStatus<V, O, M> {
    ActionHandled,
    Waiting(Vec<InputHint>),
    TextInput(KeyChord),
    None,
}
```

### TuiPages Methods

```rust
impl TuiPages<V, A, S, O, M> {
    pub fn handle_key(&mut self, event: KeyEvent, state: &mut S) -> Result<TuiPagesOutput<V, O, M>, TuiPagesError>;

    pub fn submit_command(&mut self, input: &str, state: &mut S) -> Result<TuiPagesOutput<V, O, M>, TuiPagesError>;

    pub fn current_view(&self) -> &V;
    pub fn focus(&self) -> &FocusManager<O, M>;
    pub fn focus_mut(&mut self) -> &mut FocusManager<O, M>;
}
```

### Mode Constants

```rust
pub mod modes {
    pub const GENERAL: ModeId;
    pub const NORMAL: ModeId;
    pub const INSERT: ModeId;
    pub const SELECT: ModeId;
    pub const COMMAND: ModeId;
    pub const COMMON: ModeId;
    pub const GLOBAL: ModeId;
}
```

These are the modes the runtime manages itself. A `ModeId` is a plain string
key, so define your own for custom components — e.g.
`ModeId::borrowed("picker")` — and register bindings with `.bind(mode, …)` /
`.keymap(mode, …)`. The library does not hardcode any component mode.

### TuiEffect

```rust
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
```

### ActionOutcome

```rust
impl<V, O, M> ActionOutcome<V, O, M> {
    pub fn none() -> Self;
    pub fn effect(effect: TuiEffect<V, O, M>) -> Self;
    pub fn effects(iter: impl IntoIterator<Item = TuiEffect<V, O, M>>) -> Self;
}
```

## Focus Module (`tui_pages::focus`)

### FocusManager

```rust
impl<O, M> FocusManager<O, M> {
    pub fn new() -> Self;
    pub fn current(&self) -> Option<&FocusTarget<O>>;
    pub fn index(&self) -> usize;
    pub fn focus_wrap(&self) -> FocusWrap;
    pub fn set_focus_wrap(&mut self, wrap: FocusWrap);
    pub fn next(&mut self);
    pub fn prev(&mut self);
    pub fn set_index(&mut self, index: usize);
    pub fn has_overlay(&self) -> bool;
}
```

### FocusTarget

```rust
pub enum FocusTarget<O = ()> {
    Button(usize),
    Overlay(O),
    ModalItem(usize),
    Section { id: usize, item_index: usize, item_count: usize },
    Canvas,
}
```

### FocusIntent

```rust
pub enum FocusIntent<O, M> {
    Next,
    Prev,
    Set(usize),
    Open(OverlayFocus<O, M>),
    Close,
    Toggle,
}
```

### FocusWrap

```rust
pub enum FocusWrap {
    Clamp,  // Default
    Wrap,
}
```

## Input Module (`tui_pages::input`)

### KeyChord

```rust
impl KeyChord {
    pub fn from_event(event: &KeyEvent) -> Self;
    pub fn parse(s: &str) -> Result<Self, ParseKeyError>;
}

impl std::fmt::Display for KeyChord;
impl std::str::FromStr for KeyChord;
```

### parse_binding

```rust
pub fn parse_binding(s: &str) -> KeyChord;
pub fn try_parse_binding(s: &str) -> Result<KeyChord, ParseKeyError>;
```

### InputRegistry

```rust
impl<A> InputRegistry<A> {
    pub fn new() -> Self;
    pub fn bind(&mut self, mode: ModeId, chord: KeyChord, action: A);
    pub fn bind_sequence(&mut self, mode: ModeId, chords: Vec<KeyChord>, action: A);
    pub fn match_action(&self, chords: &[KeyChord], modes: &[&str]) -> Option<A>;
    pub fn get_hints(&self, chords: &[KeyChord], modes: &[&str]) -> Vec<InputHint>;
}
```

## Command Module (`tui_pages::command`)

### CommandRegistry

```rust
impl<A> CommandRegistry<A> {
    pub fn new() -> Self;
    pub fn register(&mut self, name: &str, aliases: Vec<&str>, action: A);
    pub fn match_action(&self, input: &str) -> Option<A>;
    pub fn get_hints(&self, input: &str) -> Vec<CommandHint>;
    pub fn is_prefix(&self, input: &str) -> bool;
}
```

### CommandResponse

```rust
pub enum CommandResponse<A> {
    Execute(A),
    Incomplete(Vec<CommandHint>),
    Unknown,
    Empty,
}
```

## Navigation Module (`tui_pages::navigation`)

### BufferState

```rust
impl<V> BufferState<V> {
    pub fn new(view: V) -> Self;
    pub fn current(&self) -> Option<&V>;
    pub fn push(&mut self, view: V);
    pub fn previous(&mut self) -> Option<&V>;
    pub fn next(&mut self) -> Option<&V>;
    pub fn can_go_back(&self) -> bool;
    pub fn can_go_forward(&self) -> bool;
}
```

### PaneSplit

```rust
pub enum PaneSplit {
    Horizontal,
    Vertical,
}
```

## Dialog Module (`tui_pages::dialog`) — Feature: `dialog`

### DialogData

```rust
pub trait DialogData {
    type Result;
    fn title(&self) -> &str;
    fn item(&self, index: usize) -> &str;
    fn item_count(&self) -> usize;
}
```

### DialogResult

```rust
pub enum DialogResult<T> {
    Selected(T),
    Cancelled,
}
```

### DialogKey

```rust
pub enum DialogKey {
    Consumed,
    Ignored,
    Resolved(DialogResult<T>),
}
```

### Helper Functions

```rust
pub fn handle_key<O, M>(focus: &mut FocusManager<O, M>, key: KeyEvent) -> Result<DialogKey, TuiPagesError>;

pub fn render_dialog<F, D: DialogData>(
    frame: &mut Frame<F>,
    dialog: &D,
    theme: DialogTheme,
    area: Rect,
);
```
