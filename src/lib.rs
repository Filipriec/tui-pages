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
#[cfg(feature = "canvas")]
pub mod canvas;
#[cfg(feature = "dialog")]
pub mod dialog;
pub mod emacs;
pub mod focus;
pub mod helix;
pub mod input;
pub mod keybind;
pub mod navigation;
pub mod runtime;
pub mod terminal;
pub mod vim;

#[cfg(feature = "dialog")]
pub use dialog::{render_dialog, DialogData, DialogKey, DialogResult, DialogTheme};

pub use command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
#[cfg(feature = "canvas")]
pub use crate::canvas::{
    accepts_text_input as canvas_accepts_text_input, dispatch_action as dispatch_canvas_action,
    dispatch_key_event as dispatch_canvas_key_event,
    dispatch_text_area_key as dispatch_canvas_text_area_key,
    dispatch_text_input_key as dispatch_canvas_text_input_key,
    focus_intent_for_boundary as canvas_focus_intent_for_boundary,
    mode_for_app_mode as canvas_mode_for_app_mode, modes_for_app_mode as canvas_modes_for_app_mode,
    render_canvas_with_suggestions, render_canvas_with_suggestions_default,
    text_chord_to_action as canvas_text_chord_to_action,
    text_chord_to_canvas_action, update_cursor_style_for_editor,
    update_cursor_style_for_mode, BoundaryExit as CanvasBoundaryExit, CanvasAction,
    CanvasDispatchOutcome, CanvasKeyDispatchOutcome, CanvasTextWidgetOutcome,
};
pub use emacs::{
    bind_emacs_general_defaults, bind_emacs_global_defaults, bind_emacs_navigation_defaults,
};
pub use focus::{
    FocusController, FocusIntent, FocusManager, FocusQuery, FocusTarget, FocusWrap, Focusable,
    OverlayFocus, PageFocusBuilder,
};
pub use helix::{
    bind_helix_general_defaults, bind_helix_global_defaults, bind_helix_navigation_defaults,
};
pub use input::{
    parse_binding, parse_key, try_parse_binding, try_parse_key, ChordSequenceTracker, InputHint,
    InputPipeline, InputRegistry, KeyChord, KeyMap, ParseKeyError, PipelineResponse,
};
pub use keybind::{
    navigation_action_outcome, try_standard_navigation_action, NavigationAction,
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
pub use vim::{
    bind_vim_general_defaults, bind_vim_global_defaults, bind_vim_navigation_defaults,
    try_standard_vim_action, vim_action_outcome, VimAction,
};

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
    pub use crate::terminal;
    pub use crate::{
        modes, parse_binding, try_parse_binding, ActionContext, ActionOutcome, FocusController,
        FocusIntent, FocusManager, FocusQuery, FocusTarget, FocusWrap, KeyChord, ModeId,
        NavigationAction, PageFn, PageFocusBuilder, PageProvider, PageSpec, ParseKeyError,
        TerminalGuard, TuiActionHandler, TuiApp, TuiEffect, TuiPages, TuiPagesOutput,
        TuiPagesStatus, VimAction,
    };
    pub use crate::{
        navigation_action_outcome, try_standard_navigation_action, try_standard_vim_action,
        vim_action_outcome,
    };

    #[cfg(feature = "dialog")]
    pub use crate::dialog::{self, DialogData, DialogKey, DialogResult, DialogTheme};
    #[cfg(feature = "dialog")]
    pub use crate::render_dialog;
    #[cfg(feature = "canvas")]
    pub use crate::canvas::{
        dispatch_action as dispatch_canvas_action, dispatch_key_event as dispatch_canvas_key_event,
        dispatch_text_area_key as dispatch_canvas_text_area_key,
        dispatch_text_input_key as dispatch_canvas_text_input_key, CanvasAction,
        CanvasDispatchOutcome, CanvasKeyDispatchOutcome, CanvasTextWidgetOutcome,
    };
}
