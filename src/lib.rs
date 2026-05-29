//! Coordination runtime for keyboard-driven, page-based TUI applications.
//!
//! The crate is intentionally domain-agnostic: applications own their action
//! enum, view/page enum, and state. The library owns the coordination model —
//! input sequences, command resolution, focus, overlays, navigation, buffers,
//! and panes — and applies the [`TuiEffect`] values an application returns.
//!
//! [`TuiPages`] is the primary entry point. The submodules ([`input`],
//! [`command`], [`focus`], [`navigation`]) expose the same primitives for
//! advanced callers that want to wire the flow themselves.

pub mod command;
#[cfg(feature = "dialog")]
pub mod dialog;
pub mod focus;
pub mod input;
pub mod navigation;
pub mod runtime;

#[cfg(feature = "dialog")]
pub use dialog::{render_dialog, DialogData, DialogResult, DialogTheme};

pub use command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
pub use focus::{
    FocusController, FocusIntent, FocusManager, FocusQuery, FocusTarget, Focusable, OverlayFocus,
    OverlayKind, PageFocusBuilder,
};
pub use input::{
    parse_binding, parse_key, ChordSequenceTracker, InputHint, InputPipeline, InputRegistry, KeyChord, KeyMap,
    PipelineResponse,
};
pub use navigation::{
    BufferState, NavigationCoordinator, NavigationEvent, NavigationResult, NavigationRouter,
    PaneId, PaneSession, PaneSplit, ViewBuffer, WorkspaceState,
};
pub use runtime::{
    modes, ActionContext, ActionOutcome, ModeId, PageProvider, PageSpec, TuiActionHandler,
    TuiEffect, TuiPages, TuiPagesBuilder, TuiPagesError, TuiPagesOutput, TuiPagesResult,
    TuiPagesStatus,
};
