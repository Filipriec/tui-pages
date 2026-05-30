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
pub mod terminal;

#[cfg(feature = "dialog")]
pub use dialog::{render_dialog, DialogData, DialogKey, DialogResult, DialogTheme};

pub use command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
pub use focus::{
    FocusController, FocusIntent, FocusManager, FocusQuery, FocusTarget, FocusWrap, Focusable,
    OverlayFocus, PageFocusBuilder,
};
pub use input::{
    parse_binding, parse_key, try_parse_binding, try_parse_key, ChordSequenceTracker, InputHint,
    InputPipeline, InputRegistry, KeyChord, KeyMap, ParseKeyError, PipelineResponse,
};
pub use navigation::{
    BufferState, NavigationCoordinator, NavigationEvent, NavigationResult, NavigationRouter,
    PaneId, PaneSession, PaneSplit, ViewBuffer, WorkspaceState,
};
pub use runtime::{
    modes, ActionContext, ActionOutcome, ModeId, PageFn, PageProvider, PageSpec, TuiActionHandler,
    TuiApp, TuiEffect, TuiPages, TuiPagesBuilder, TuiPagesError, TuiPagesOutput, TuiPagesResult,
    TuiPagesStatus,
};
pub use terminal::{enter as enter_terminal, TerminalGuard};

/// Everything a typical application needs in one glob import.
///
/// ```ignore
/// use tui_pages::prelude::*;
/// ```
///
/// This pulls in the runtime, the focus types, and — crucially — the
/// [`FocusController`] trait, whose [`apply_focus_intent`](FocusController::apply_focus_intent)
/// method is otherwise invisible until the trait is in scope. With the
/// `dialog` feature it also brings in the dialog content, result, theme,
/// renderer, and the `dialog::*` driver helpers.
pub mod prelude {
    pub use crate::{
        modes, parse_binding, try_parse_binding, ActionContext, ActionOutcome, FocusController,
        FocusIntent, FocusManager, FocusQuery, FocusTarget, FocusWrap, KeyChord, ModeId, PageFn,
        PageFocusBuilder, PageProvider, PageSpec, ParseKeyError, TerminalGuard, TuiActionHandler,
        TuiApp, TuiEffect, TuiPages, TuiPagesOutput, TuiPagesStatus,
    };
    pub use crate::terminal;

    #[cfg(feature = "dialog")]
    pub use crate::dialog::{self, DialogData, DialogKey, DialogResult, DialogTheme};
    #[cfg(feature = "dialog")]
    pub use crate::render_dialog;
}
