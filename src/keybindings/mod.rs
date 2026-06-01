//! Editor-style default keybinding presets (Vim, Emacs, Helix, …).
//!
//! All preset sources live in this module directory. See [`README.md`](README.md) in this folder.
//!
//! # Usage
//!
//! ```ignore
//! use tui_pages::prelude::*;
//!
//! TuiPages::builder(view)
//!     .vim_defaults()   // or .emacs_defaults() / .helix_defaults() — pick one
//!     .build();
//! ```

mod action;
mod builtin;
mod preset;

pub use action::{
    navigation_action_infos, navigation_action_outcome, try_standard_navigation_action,
    NavigationAction, NavigationActionInfo,
};
pub use builtin::{
    bind_builtin_general_defaults, bind_builtin_global_defaults,
    bind_builtin_navigation_defaults, bind_emacs_general_defaults, bind_emacs_global_defaults,
    bind_emacs_navigation_defaults, bind_helix_general_defaults, bind_helix_global_defaults,
    bind_helix_navigation_defaults, bind_vim_general_defaults, bind_vim_global_defaults,
    bind_vim_navigation_defaults, emacs_preset_toml, helix_preset_toml,
    try_standard_vim_action, vim_action_outcome, vim_preset_toml, BuiltinNavigationPreset,
    VimAction,
};
pub use preset::{
    apply_navigation_preset_toml, remap_navigation_preset_toml, NavigationPreset,
    NavigationConflictPolicy, NavigationPresetBinding, NavigationPresetError,
    NavigationPresetIssue, NavigationPresetSection,
};
