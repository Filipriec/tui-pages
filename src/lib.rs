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

#[cfg(feature = "canvas")]
pub mod canvas;
pub mod command;
#[cfg(feature = "tui")]
pub mod dialog;
pub mod focus;
pub mod input;
pub mod keybindings;
pub mod navigation;
#[cfg(feature = "tui")]
pub mod picker;
pub mod runtime;
pub mod terminal;

#[cfg(feature = "tui")]
pub use dialog::{DialogData, DialogKey, DialogResult, DialogTheme, render_dialog};

#[cfg(feature = "tui")]
pub use picker::{
    PickerData, PickerEntry, PickerField, PickerFieldWeights, PickerHead, PickerHeadColumn,
    PickerLayout, PickerScope, PickerTheme, centered_picker_area, render_picker,
    render_picker_with_custom_preview,
};

#[cfg(feature = "canvas")]
pub use crate::canvas::{
    BoundaryExit as CanvasBoundaryExit, CanvasAction, CanvasDispatchOutcome,
    CanvasKeyDispatchOutcome, CanvasTextWidgetOutcome,
    accepts_text_input as canvas_accepts_text_input, dispatch_action as dispatch_canvas_action,
    dispatch_key_event as dispatch_canvas_key_event,
    dispatch_text_area_key as dispatch_canvas_text_area_key,
    dispatch_text_input_key as dispatch_canvas_text_input_key,
    focus_intent_for_boundary as canvas_focus_intent_for_boundary,
    mode_for_app_mode as canvas_mode_for_app_mode, modes_for_app_mode as canvas_modes_for_app_mode,
    render_canvas_with_suggestions, render_canvas_with_suggestions_default,
    render_canvas_with_suggestions_default_options, render_canvas_with_suggestions_with_options,
    text_chord_to_action as canvas_text_chord_to_action, text_chord_to_canvas_action,
    update_cursor_style_for_editor, update_cursor_style_for_mode,
};
#[cfg(feature = "canvas")]
pub use crate::canvas::{
    analyze_canvas_overlaps, canvas_action_name, canvas_bindable_actions,
    canvas_default_binding_catalog, canvas_suggestion_default_bindings,
};
pub use command::{CommandHint, CommandRegistry, CommandResolver, CommandResponse};
pub use focus::{
    FocusController, FocusIntent, FocusManager, FocusQuery, FocusTarget, FocusWrap, Focusable,
    OverlayFocus, PageFocusBuilder,
};
pub use input::{
    BindableActionInfo, BindingAnalysis, BindingCatalog, BindingConflict, BindingInfo,
    BindingLayer, BindingSource, CanvasRoutingPrecedence, ChordSequenceTracker, InputHint,
    InputPipeline, InputRegistry, KeyChord, KeyMap, ParseKeyError, PipelineResponse,
    analyze_keymap_bindings, navigation_bindable_actions, parse_binding, parse_key,
    try_parse_binding, try_parse_key,
};
pub use keybindings::{
    ActionRegistry, BindingNotice, BindingStore, BuiltinNavigationPreset, KeybindingConfig,
    KeybindingConfigError, KeybindingReport, NavigationAction, NavigationActionInfo,
    NavigationConflictPolicy, NavigationPreset, NavigationPresetBinding, NavigationPresetError,
    NavigationPresetIssue, NavigationPresetSection, ParseBuiltinNavigationPresetError,
    ParseNavigationActionError, VimAction, apply_navigation_preset_toml,
    bind_builtin_general_defaults, bind_builtin_global_defaults, bind_builtin_navigation_defaults,
    bind_emacs_general_defaults, bind_emacs_global_defaults, bind_emacs_navigation_defaults,
    bind_helix_general_defaults, bind_helix_global_defaults, bind_helix_navigation_defaults,
    bind_vim_general_defaults, bind_vim_global_defaults, bind_vim_navigation_defaults,
    emacs_preset_toml, helix_preset_toml, navigation_action_infos, navigation_action_outcome,
    remap_navigation_preset_toml, try_standard_navigation_action, try_standard_vim_action,
    vim_action_outcome, vim_preset_toml,
};
pub use navigation::{
    BufferState, NavigationCoordinator, NavigationEvent, NavigationResult, NavigationRouter,
    PaneId, PaneSession, PaneSplit, ViewBuffer, WorkspaceState,
};
#[cfg(feature = "command-line")]
pub use runtime::CommandLineAreas;
pub use runtime::{
    ActionContext, ActionOutcome, InputLayerContext, ModeId, PageFn, PageProvider, PageSpec,
    RuntimeContext, TuiActionHandler, TuiApp, TuiEffect, TuiPages, TuiPagesBuilder, TuiPagesError,
    TuiPagesOutput, TuiPagesResult, TuiPagesStatus, modes,
};
pub use terminal::{TerminalGuard, enter as enter_terminal};

/// Everything a typical application needs in one glob import.
///
/// ```ignore
/// use tui_pages::prelude::*;
/// ```
///
/// This pulls in the runtime, the focus types, and — crucially — the
/// [`FocusController`] trait, whose [`apply_focus_intent`](FocusController::apply_focus_intent)
/// method is otherwise invisible until the trait is in scope. With the
/// `tui` feature it also brings in the dialog and picker content, result,
/// theme, and renderer types, plus the `dialog::*` driver helpers.
pub mod prelude {
    pub use crate::terminal;
    pub use crate::{
        ActionContext, ActionOutcome, ActionRegistry, BindableActionInfo, BindingNotice,
        BindingStore, BuiltinNavigationPreset, FocusController, FocusIntent, FocusManager,
        FocusQuery, FocusTarget, FocusWrap, KeyChord, KeybindingConfig, KeybindingConfigError,
        KeybindingReport, ModeId, NavigationAction, NavigationActionInfo, NavigationConflictPolicy,
        NavigationPreset, NavigationPresetError, NavigationPresetIssue, PageFn, PageFocusBuilder,
        PageProvider, PageSpec, ParseBuiltinNavigationPresetError, ParseKeyError, RuntimeContext,
        TerminalGuard, TuiActionHandler, TuiApp, TuiEffect, TuiPages, TuiPagesOutput,
        TuiPagesStatus, VimAction, modes, parse_binding, try_parse_binding,
    };
    pub use crate::{
        apply_navigation_preset_toml, emacs_preset_toml, helix_preset_toml,
        navigation_action_infos, navigation_action_outcome, remap_navigation_preset_toml,
        try_standard_navigation_action, try_standard_vim_action, vim_action_outcome,
        vim_preset_toml,
    };

    #[cfg(feature = "canvas")]
    pub use crate::canvas::{
        CanvasAction, CanvasDispatchOutcome, CanvasKeyDispatchOutcome, CanvasTextWidgetOutcome,
        dispatch_action as dispatch_canvas_action, dispatch_key_event as dispatch_canvas_key_event,
        dispatch_text_area_key as dispatch_canvas_text_area_key,
        dispatch_text_input_key as dispatch_canvas_text_input_key,
    };
    #[cfg(feature = "tui")]
    pub use crate::dialog::{
        self, DialogData, DialogKey, DialogKeyBindings, DialogResult, DialogTheme,
    };
    #[cfg(feature = "tui")]
    pub use crate::render_dialog;

    #[cfg(feature = "tui")]
    pub use crate::picker::{
        PickerData, PickerEntry, PickerField, PickerFieldWeights, PickerHead, PickerHeadColumn,
        PickerLayout, PickerScope, PickerTheme, centered_picker_area, render_picker,
        render_picker_with_custom_preview,
    };
}
