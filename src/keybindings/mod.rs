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
mod emacs;
mod helix;
mod vim;

pub use action::{
    navigation_action_outcome, try_standard_navigation_action, NavigationAction,
};
pub use emacs::{
    bind_emacs_general_defaults, bind_emacs_global_defaults, bind_emacs_navigation_defaults,
};
pub use helix::{
    bind_helix_general_defaults, bind_helix_global_defaults, bind_helix_navigation_defaults,
};
pub use vim::{
    bind_vim_general_defaults, bind_vim_global_defaults, bind_vim_navigation_defaults,
    try_standard_vim_action, vim_action_outcome, VimAction,
};
