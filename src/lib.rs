//! Reusable architecture primitives for keyboard-driven TUI applications.
//!
//! The crate is intentionally domain-agnostic: applications provide their own
//! action enum, page/view enum, command aliases, and page handlers.

pub mod action;
pub mod command;
pub mod focus;
pub mod input;
pub mod navigation;

pub use action::{
    ActionDecider, ActionDispatcher, ActionResolution, DefaultActionDecider, PageActionHandler,
    PageActionResult, RoutedAction,
};
pub use command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
pub use focus::{
    FocusController, FocusIntent, FocusManager, FocusQuery, FocusTarget, Focusable, KeyMode,
    ModeHint, OverlayFocus, OverlayKind, PageFocusBuilder,
};
pub use input::{
    parse_binding, parse_key, ChordSequenceTracker, InputHint, InputPipeline, InputRegistry, KeyChord, KeyMap,
    PipelineResponse,
};
pub use navigation::{
    BufferState, NavigationCoordinator, NavigationEvent, NavigationResult, NavigationRouter,
    PaneId, PaneSession, PaneSplit, ViewBuffer, WorkspaceState,
};
